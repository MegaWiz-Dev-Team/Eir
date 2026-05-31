# Design — Medical Agent/Skill Data Model & Migration

- **Status:** Draft / Proposed
- **Date:** 2026-05-22
- **Decision:** [ADR-010](../../../Asgard/docs/decisions/ADR-010-agents-as-boundaries-skills-as-expertise.md)
- **Architecture:** `medical-agent-architecture.md`; **Runtime:**
  `Bifrost/docs/design/skill-loader-runtime.md`
- **Schema home:** Mimir (`ro-ai-bridge/migrations/`). Proposed DDL below is
  **design, not an applied migration** — a real `sprintNN_agent_skills.sql`
  lands only when ADR-010 is Accepted.

This note specifies the storage changes that turn "19 cloned agents" into
"5 boundary agents + a skill registry."

---

## 1. Current schema (verified)

```sql
-- mimir-core-ai/migrations/.../sprint13_agent_studio.sql
agent_configs(
  id, tenant_id, name, display_name, system_prompt, model_id, provider,
  temperature, max_tokens, tools JSON,  -- "Array of enabled tool names"
  ...
)
-- ro-ai-bridge/migrations/sprint38_specialty_agents.sql  (added)
  + specialty VARCHAR(40), is_router TINYINT, routes_to_specialties JSON
  + INDEX (tenant_id, specialty)
```

`tools` already means "the tools this agent may use" — under ADR-010 it becomes
the **enforced tool ceiling**. No rename needed; we change *enforcement*, not the
column.

---

## 2. Changes to `agent_configs` (the boundary agents)

```sql
ALTER TABLE agent_configs
  ADD COLUMN safety_class    VARCHAR(24)  NOT NULL DEFAULT 'standard',
      -- standard | safety_critical    (drives HITL + refusals; see behaviour table TBD)
      -- 'restricted' is reserved for the deferred forensic boundary, unused for now
  ADD COLUMN allowed_models  JSON         NULL,
      -- list of LOCAL model_ids a skill's model_hint may pin to; NULL = [model_id] only
  ADD COLUMN skill_policy    JSON         NULL;
      -- optional: { "allow_specialties": [...], "deny_specialties": [...] }
      -- NULL = any skill may load (gated only by the tool ceiling)

-- NOTE: access_scope is NOT added yet. The only consumer was eir-forensic, which
-- is deferred (ADR-009 = no platform RBAC, so we can't enforce it on-box). Add
-- access_scope when forensic access control is designed at the Eir Gateway layer.

-- `tools` (existing column) IS the tool CEILING — the enforced maximum. There is
-- no separate "base" column; the loader narrows the exposed set per turn from
-- active skills, never exceeding `tools`.
-- system_prompt for boundary agents holds the BASE CoT only (no specialty preamble).

-- Deprecations (keep columns for one release for rollback, stop writing them):
--   is_router            → no router; agent selection = deterministic resolver (arch §4b)
--   routes_to_specialties→ unused
--   specialty            → vestigial for boundary agents (specialty lives on skills)
```

`safety_class` + `allowed_models` are the enforceable boundary attributes the
loader reads (skill-loader-runtime §3.4) and the dispatch layer checks. The
**5 boundary agents** are `eir-clinical`, `eir-pharmacy`, `eir-pediatrics`,
`eir-psychiatry`, `eir-emergency` (forensic deferred).

---

## 3. New: `agent_skills` (the registry)

One row per expertise module. Shared/global by default (`tenant_id IS NULL`),
mirroring the shared-KB convention.

```sql
CREATE TABLE agent_skills (
  skill_id        VARCHAR(80)  NOT NULL,         -- slug, e.g. 'cardio-acs-workup'
  name            VARCHAR(200) NOT NULL,
  description     TEXT         NOT NULL,         -- the ONLY field embedded for selection
  specialty       VARCHAR(40)  NULL,             -- cardiology | ent | …  (for filtering/MMR)
  reasoning_frame MEDIUMTEXT   NULL,             -- the ex-preamble domain CoT
  allowed_tools   JSON         NOT NULL,         -- ⊆ host agent ceiling (narrow-only)
  knowledge_scope JSON         NULL,             -- { collection, filter:{...} }
  model_hint      VARCHAR(100) NULL,             -- must ∈ host agent.allowed_models else ignored; NULL = inherit
  safety_flags    JSON         NULL,             -- ["require_hitl", ...]
  source          VARCHAR(40)  NULL,             -- 'medopenclaw' | 'clinical-wisdom'
  version         VARCHAR(32)  NOT NULL DEFAULT '1',
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',  -- active|draft|retired
  tenant_id       VARCHAR(50)  NULL,             -- NULL = shared
  created_at      TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
  updated_at      TIMESTAMP    DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (skill_id, version),               -- versioned; latest = MAX(version) where active
  KEY idx_skill_specialty (specialty, status),
  KEY idx_skill_status    (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

**Selection index (Qdrant):** collection `skills-catalog`, one point per active
skill, vector = BGE-M3 embedding of `description`, payload = `{skill_id,
specialty, status}`. This is what `POST /api/v1/skills/select` searches. (B-49b
already produced these embeddings; this formalizes where they live.)

> **Skill ≠ Tool.** A *tool* is an executable endpoint in the Hermodr/Mimir
> catalog (e.g. `search_primekg`). A *skill* is a reasoning module that *declares
> which tools it uses* via `allowed_tools`. The catalog of tools is designed in
> the companion knowledge-&-tool-layer note.

---

## 4. Optional: `agent_skill_activations` (audit/telemetry)

Not required for function; recommended for the parity test + Tyr trail.

```sql
CREATE TABLE agent_skill_activations (
  id          BIGINT AUTO_INCREMENT PRIMARY KEY,
  request_id  VARCHAR(64) NOT NULL,
  agent_id    VARCHAR(100) NOT NULL,
  skill_id    VARCHAR(80)  NOT NULL,
  score       FLOAT        NULL,
  tenant_id   VARCHAR(50)  NULL,
  created_at  TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
  KEY idx_req (request_id)
);
```

(Or emit straight to heimdall-trace/Tyr instead of a table — decide in §6 of the
runtime doc. Pick one sink, not both.)

---

## 5. Ingest: MedOpenClaw `SKILL.md` → `agent_skills`

Maps the existing frontmatter to columns (extends B-49b catalog ingest):

| `SKILL.md` field | `agent_skills` column |
|------------------|-----------------------|
| `name` (slug)    | `skill_id`            |
| `description`    | `description` (+ embed → `skills-catalog`) |
| `allowed-tools`  | `allowed_tools` (validated ⊆ some agent ceiling) |
| (body / SOP)     | `reasoning_frame`     |
| `tool_type` / domain | `specialty` (derive/curate) |

Ingest is idempotent on `(skill_id, version)`. A skill whose `allowed_tools`
contains a tool no boundary agent ceiling offers is ingested as `status='draft'`
(can't be safely activated) and flagged for curation.

---

## 6. Migration: 19 agents → 5 boundary agents + skills

**Forward, reversible, data-preserving:**

1. **Snapshot** the 19 `agent_configs` rows (rollback source).
2. **Define the 5 boundary agents** (insert/keep): `eir-clinical`,
   `eir-pharmacy`, `eir-pediatrics`, `eir-psychiatry`, `eir-emergency`. Set
   `system_prompt = base CoT`, `tools = ceiling`, `allowed_models`, `safety_class`.
   (`eir-forensic` is **deferred** — not created; see arch §3/§12.)
3. **Port the ~13 persona-only specialists → `agent_skills`** (incl. forensic's
   *reasoning* content held back, not ported, until access control exists): for
   each, extract its specialty preamble → `reasoning_frame`, its allowlist →
   `allowed_tools` (must be ⊆ host agent ceiling, else `status='draft'`), set
   `specialty`, `source='clinical-wisdom'`, `status='active'`; embed
   `description` → `skills-catalog`.
4. **Retire the router:** the `is_router=1` row → `status` retired (or delete
   after the parity test). Stand up the deterministic agent-resolver (arch §4b)
   in its place.
5. **Backfill `safety_class`** (peds/psychiatry → `safety_critical`, rest →
   `standard`).
6. **Gate behind a flag** (`USE_SKILL_LOADER`) so the old per-specialty path and
   the new loader can run side-by-side for the parity test (architecture doc §11).

**Rollback:** flip the flag off + restore the snapshot. Because skills are
*additive* and the boundary agents are a superset, the old behaviour is
recoverable until the per-specialty rows are deleted (step 4, do last).

---

## 7. Open questions

- `model_hint` validation: it must be a member of the host agent's
  `allowed_models` (checked at ingest *and* at load — defence in depth). Where is
  the canonical per-agent local-model allowlist seeded from?
- Do we need per-tenant skill overrides (`tenant_id != NULL` rows shadowing a
  shared skill), or are all skills shared on a single-tenant box?
- `specialty` taxonomy for skills vs the `knowledge_scope.filter` tags on
  clinical-wisdom chunks — same vocabulary? (see knowledge-&-tool-layer note).
- Versioning: is "latest active = MAX(version)" enough, or do we need an explicit
  `is_current` flag for fast lookups?
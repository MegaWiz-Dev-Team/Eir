# Design — Medical AI Agent Architecture: Agents as Boundaries, Skills as Expertise

- **Status:** Draft / Proposed
- **Date:** 2026-05-22
- **Refines:** `Eir_Agents_Architecture.md` §3 (the "19 Eir agents" roster) and
  §4.5 (Bifrost specialty router). Keeps §2 (single-tenant per Mac mini), the
  LOCAL-LLM-only rule, and the multidisciplinary-team concept (§1) unchanged.
- **Spans:** Eir (personas), Bifrost overseer (runtime), Mimir (`agent_configs`
  + skill registry), MedOpenClaw (skill source), Hermodr (tool catalog),
  Skuggi (safety), Tyr (audit).

---

## 1. The decision

> **An Agent is a *trust & policy boundary* — few, stable, enforceable.
> A Skill is an *expertise module* — many, composable, retrieved at request
> time. Map a medical specialty to a *skill* by default; promote it to its own
> *agent* only when it needs a distinct enforceable boundary (tool allowlist,
> model, safety floor, or access restriction).**

This replaces "one cloned agent per specialty (19 → 28)" with **5 boundary
agents + a skill library** (the 869 MedOpenClaw skills + clinical-wisdom).

### Decision rule (apply per specialty)

```
Does this specialization change WHAT THE AGENT MAY TOUCH / WHICH MODEL /
WHO MAY ACCESS / WHAT IT MUST REFUSE?
   YES → it is an AGENT boundary
   NO  → it is "how to reason about a domain / which SOP to follow"
        → it is a SKILL
```

---

## 2. Why — grounded in our own evidence

1. **Persona-routing gave ~0pp lift.** `Eir_Agents_Architecture.md` §5.1 records
   the Sprint 38f A/B result: router-vs-monolithic **Δ = +0.0pp HBp** on the
   general mix; ROI only on specialty-tagged subsets; the 28-specialty expansion
   was aborted. Cloning a persona is not where the value is.
2. **Most of the 19 agents are tool-identical.** In §3.1/§3.2, ~13 of 19 carry
   the *same* allowlist (`search_primekg, search_clinical_kb, read_fhir,
   pubmed_search`) and differ only in preamble. They are skills wearing agent
   costumes — each still pays ~1,500–1,750 prompt tokens and a clone to maintain.
3. **Routing cost & a policy violation.** The LLM specialty router adds
   ~150–400ms/req, and the Sprint 38 PoC ran sleep/ENT/peds on **cloud Gemini**,
   violating the Eir local-only rule. Skill *retrieval* (embeddings, already
   built in B-49b) removes both problems.
4. **Long tail doesn't fit agents.** 869 OpenClaw skills cannot be 869
   `agent_configs` rows behind an 869-class classifier. Skills scale; agents
   don't.

---

## 3. Agent layer — the few (boundaries)

Proposed roster for `asgard_medical`. Each agent is justified by a **boundary
that must be enforced**, not by a clinical label. Default model stays local.

| Agent | Boundary that justifies it | Model | Hard tool ceiling (superset) |
|-------|----------------------------|-------|------------------------------|
| **eir-clinical** | General reasoning host; default. Carries most specialties **as skills**. | gemma-4-26b | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search`, `clinical_calculator` |
| **eir-pharmacy** | Tool + safety boundary: DDI / dosing / formulary. **Mandatory gate** on any prescription action. | gemma-4-26b | + `drug_interaction_check`, `dosage_calculator`, `formulary_lookup`, `drug_food_interaction` |
| **eir-pediatrics** | Safety boundary: age/weight dosing must be guaranteed, not hoped. | medgemma-27b-text | `read_fhir`, `search_clinical_kb`, `dosage_calculator (pediatric)`, `pubmed_search` |
| **eir-psychiatry** | Safety floor: hard-refuse suicide-method requests; psychotropic DDI. | medgemma-27b-text | + `drug_interaction_check`, `clinical_calculator (PHQ-9/GAD-7)` |
| **eir-emergency** | Latency class (≤2s p50) + triage tools. | gemma-4-26b Q4 | `search_clinical_kb`, `read_fhir`, `clinical_calculator (Wells/NEXUS/GCS)`, `triage_score` |

**5 boundary agents** (was 6). **`eir-forensic` is deferred** — its only
justification is *access restriction* (forensic team only), but ADR-009 mandates
**no platform RBAC** on a single-tenant box, so that boundary is **not
enforceable on-platform today**. Per our own "agents = *enforceable* boundaries"
rule, forensic is neither an agent nor a skill until access control is designed
at the Eir Gateway (OpenEMR) layer (it has user roles); exposing autopsy/forensic
reasoning to everyone is itself a risk. Tracked in §12.

**The other 13 specialties become skills** on `eir-clinical` (or the relevant
boundary agent): internal-medicine, surgery, ophthalmology, orthopedics, ENT,
urology, radiology(text), nursing, medtech, PT, dietitian, social-work, ob-gyn.

> Borderline cases: **dietitian** (`nutrition_calculator`, `drug_food_interaction`)
> and **ob-gyn** (pregnancy-category DDI) need a tool or two beyond the
> `eir-clinical` ceiling. Resolve by either (a) adding those tools to the
> `eir-clinical` ceiling and gating them inside the skill's `allowed-tools`, or
> (b) delegating the regulated step to `eir-pharmacy`. Prefer (b) for anything
> safety-critical (pregnancy DDI → pharmacy).

**Agent record (extends `agent_configs`):** keep `system_prompt` to the *base
CoT only* (no per-specialty preamble); the differentiators are `model_id`, the
enforced **tool ceiling** (`tools` — the maximum set; effective per-turn tools
are computed by the loader, never exceeding it), an `allowed_models` list (the
local models a skill `model_hint` may pick from), and a `safety_class`.
`is_router` is **deprecated** (replaced by §4b). `access_scope` is **not added
yet** — deferred with `eir-forensic` (no platform RBAC to enforce it).

---

## 4. Skill layer — the many (expertise)

A **skill** is a retrievable module of domain procedure, not an identity.

**Schema** (extends the MedOpenClaw `SKILL.md` frontmatter that already exists):

```yaml
---
name: cardio-acs-workup
description: <one-liner; THIS is what the retriever embeds & matches>
specialty: cardiology
reasoning_frame: |
  <the domain CoT framing that used to be a per-agent preamble —
   "hemodynamics → mechanism → guideline-based therapy", red flags, etc.>
allowed_tools:            # MUST be a subset of the host agent's ceiling
  - search_primekg
  - search_clinical_kb
  - pubmed_search
knowledge_scope:          # retrieval filter (collection + metadata tag)
  collection: clinical-wisdom
  filter: { specialty: cardiology }
model_hint: null          # null = inherit agent; else MUST be one of the host
                          #   agent's `allowed_models` (else ignored). No "safer"
                          #   ordering — just an explicit allowlist membership check.
safety_flags: []          # e.g. [require_hitl, refuse_self_harm_methods]
examples: [ ... ]
---
```

- **Source:** MedOpenClaw 869 (cataloged + embedded in B-49b) + clinical-wisdom.
- **Registry:** Mimir (`mimir-skills`), already holds metadata + embeddings.
- **Key property:** skills are **selected by retrieval**, never by an LLM router.

---

## 4b. Agent resolution — deterministic, replaces `eir-router`

Removing the LLM router solved *skill* selection (retrieval, §4) — but *agent*
selection still has to happen, and it is now **safety-critical**: routing a
pediatric case to `eir-clinical` (which cannot call `dosage_calculator`) is a
safety failure, not just a quality miss. So agent resolution is a **deterministic
rule gate that runs BEFORE the loader**, not an LLM call.

It is the surviving, safety-relevant subset of the old §4.5 routing rules,
evaluated on structured signals (not free-text classification):

```
resolve_agent(request) -> agent_id        // runs before skill loading
  if prescription/medication-order intent     → eir-pharmacy  (mandatory gate)
  if patient age < 18 (from FHIR)              → eir-pediatrics
  if mental-health context / SI / PHQ-GAD      → eir-psychiatry
  if emergency/triage entrypoint               → eir-emergency
  else                                         → eir-clinical   (default host)
```

Invariants:

- **Never downgrade silently.** If a signal is ambiguous but safety-relevant,
  escalate to the *stricter* agent (or attach the stricter agent via the
  cross-boundary fan-out in §9) — never fall back to `eir-clinical` to "be safe".
- **Deterministic + auditable.** Signals come from structured data (FHIR age,
  order intent), not an LLM guess. Every resolution is logged to Tyr.
- **Default is `eir-clinical`**, which then composes skills (§4–§5).
- Multiple boundaries can apply → §9 cross-boundary orchestration (e.g. a
  pediatric prescription resolves to `eir-pediatrics` **and** fans out to the
  mandatory `eir-pharmacy` gate).

> Where does the signal come from for the *first* turn (before any agent runs)?
> The triage/first-touch step (today's nursing flow) extracts age/intent from the
> request + FHIR. **On the safety-critical branches (peds / pharmacy / psychiatry)
> the signal MUST be structured/deterministic** — FHIR age, an explicit UI
> order-intent flag — **not NLP intent classification.** Adding an ML hop on the
> safety path re-introduces exactly the nondeterminism §4b exists to remove. A
> classifier, if any, may assist *non-safety* routing only. (Open item §12.)

---

## 5. Runtime — how one turn works (the missing piece)

The only net-new machinery is a **skill-loader in the Bifrost overseer**.
Everything else (retrieval, tools, structured output) already exists.

```
request ─▶ ① RESOLVE AGENT  (deterministic gate §4b → boundary agent)
                │  sets HARD tool ceiling + allowed_models + safety_class
                ▼
           ② SELECT SKILLS  (embedding match over skill.description, top-k +
                │            score floor — NOT an LLM call; reuses B-49b vectors)
                ▼
           ③ COMPOSE CONTEXT (progressive disclosure)
                │  base CoT + selected skills' reasoning_frame
                │  + retrieval filtered by each skill.knowledge_scope
                ▼
           ④ INTERSECT TOOLS
                │  effective = agent.ceiling ∩ (base ∪ ⋃ skill.allowed_tools)
                │  skills may only NARROW, never EXPAND the ceiling
                ▼
           ⑤ EXECUTE  (overseer loop → Mimir retrieval + Hermodr tools)
                ▼
           ⑥ GUARD + AUDIT  (Skuggi PII gate; Tyr logs skill activation +
                             every tool call; safety_flags enforced)
```

**Replaces** `eir-router` (§4.5). No specialty-classification LLM hop; selection
is vector similarity. Multiple skills can co-activate (e.g. a cardiology +
nephrology query loads both frames) — composition, not single-winner routing.

**Progressive disclosure ties into context compaction** (see Bifrost
`docs/design/agent-memory-evolution.md`): inject skill *descriptions* cheaply,
load full *bodies* only for the top matches. Under context pressure, drop the
lowest-scored skill body first — never the pinned safety frame.

---

## 6. Tool enforcement — close the current gap

Today (`overseer.rs`) an unknown tool in `agent_configs.tools` only logs a
warning; there is no server-side deny. Under this design:

- The **agent ceiling is the hard limit**, enforced at dispatch (Bifrost
  overseer + Hermodr), deny-by-default.
- A skill's `allowed_tools` can only **select within** the ceiling. A skill
  loaded onto `eir-clinical` can therefore *never* reach `dosage_calculator`,
  because that tool lives only on the `eir-pediatrics` / `eir-pharmacy` ceiling.
  **This is exactly why pediatric dosing stays an agent, not a skill** — we must
  *guarantee* it, not trust the prompt.
- Every denied attempt is an event to **Tyr**.

---

## 7. Knowledge scoping

Today all agents share the same Qdrant collections (no real per-agent
specialization). Move scoping to the **skill**: each carries a `knowledge_scope`
(collection + metadata filter). Retrieval for an active skill is filtered to its
scope, so a cardiology skill pulls cardiology-tagged chunks without standing up a
separate collection. No duplication; specialization happens at query time.

---

## 8. Safety & compliance (non-negotiable)

- **LOCAL-LLM only** stays enforced at the **agent** layer: `model_id` is
  validated against the local allowlist; cloud reasoning models are rejected for
  any Eir agent. (Collapsing cloud-Gemini sleep/ENT/pets into skills on a local
  agent *fixes* the current PoC violation.)
- **Skuggi** PII gate on inputs/outputs; **Tyr** audits skill activation + tool
  calls + erasure (treat PII/medical flows as Tyr-first, not afterthought).
- **safety_class** on the agent + `safety_flags` on the skill drive HITL and hard
  refusals (e.g. psychiatry self-harm floor, pregnancy-DDI → require pharmacy).

---

## 9. Multi-agent orchestration still holds

The team-of-specialists flows (§4) are preserved, but the distinction sharpens:

- **Within a boundary** → skill composition on one agent (e.g. internal-medicine
  + nephrology reasoning on `eir-clinical`, one call, both frames).
- **Across a boundary** → genuine multi-agent fan-out (e.g. **Pharmacy DDI is
  always invoked** on a prescription → real call to `eir-pharmacy`, because it is
  a separate tool + safety boundary). Bifrost still synthesizes + resolves
  conflicts + merges citations.

So we keep multi-agent where it buys *enforced separation*, and drop it where it
was only persona theater.

---

## 9b. Applying the model elsewhere — `asgard_platform` & the global assistant

The same rule catches an existing inconsistency (flagged in cross-session review
2026-05-22):

- **`primekg-graph-assistant` (agent id=9 @ `asgard_platform`) is the anti-pattern
  ADR-010 names.** It is graph Q&A: same model as `eir-clinical`, a tool *subset*,
  no distinct refusal/safety floor → **not a boundary**. Its only special trait is
  reading cross-tenant **shared/public** KBs — but that is read of *non-sensitive*
  data, the opposite of a restriction, and cross-tenant separation is already a
  **tenant/box boundary** under ADR-009, **not an agent boundary**.
  **Decision:** convert it to a **`graph-explorer` skill** (declares
  `primekg_resolve` / `primekg_disease_relations` / `primekg_neighbors`) on the
  `asgard_platform` tenant's **default host agent** (the platform's analogue of
  `eir-clinical`). Do not keep it as a separate agent. *(Live change to agent id=9
  — execute deliberately by its owner; this doc records the target, not the edit.)*
- **The global / platform assistant uses §4b deterministic resolution + retrieved
  skills — never an LLM "router agent".** (This explicitly retires the earlier
  "global assistant → Bifrost LLM-router" proposal, which is exactly the
  `eir-router` pattern ADR-010 deprecates. Good that it was never built.)

---

## 10. Migration

1. **Freeze agent expansion.** Do not build the remaining specialty agents
   (data already says 0pp; 28-expansion already aborted).
2. **Build the skill-loader MVP** in the Bifrost overseer: steps ②–④ (retrieve
   top-k skills, inject frames, intersect tools). Wire to the `mimir-skills`
   registry + B-49b embeddings.
3. **Port the 13 persona-only specialists → skills** (reasoning_frame +
   knowledge_scope; allowed_tools ⊆ `eir-clinical` ceiling).
4. **Keep/harden the 5 boundary agents.** Move sleep/ENT/peds off cloud Gemini
   to local models. Enforce tool ceilings server-side (§6).
5. **Deprecate `eir-router`.** Replace with skill retrieval.
6. **Scale to the long tail** — fold the rest of the 869 skills in as the loader
   proves out.

---

## 11. What to measure (gates)

- **Parity test:** skill-loader (`eir-clinical` + skills) vs the per-specialty
  agents on the HBp **specialty-tagged subset** — expect ≥ parity with less infra.
- **Skill retrieval precision@k** + co-activation correctness.
- **Latency:** confirm removal of the ~150–400ms router hop.
- **Tool-violation attempts caught** at dispatch (should be >0 and all denied).

---

## 12. Open questions

- **`eir-forensic` access control** — re-introduce as a boundary agent once the
  Eir Gateway (OpenEMR) layer enforces forensic-team access; then add the
  `access_scope` column (deferred from the data model). Until then forensic
  reasoning is intentionally absent.
- **First-turn signal extraction for §4b** — is the age/intent gate a tiny
  deterministic parser over the request+FHIR, or a small classifier? Must stay
  deterministic for the safety-critical branches (peds/pharmacy/psychiatry).
- **`safety_class` behaviour table** — define exactly what `standard` /
  `safety_critical` trigger (HITL? which refusal lists?) before phase 2.
- Top-k and score-floor for skill selection — tune against specialty-tagged
  traffic; how many skills may co-activate before context bloat?
- Skill ↔ knowledge_scope: do we tag existing clinical-wisdom chunks per
  specialty now, or derive scope from PrimeKG entity types at query time?
- Dietitian / ob-gyn tool placement — extend `eir-clinical` ceiling vs delegate
  to pharmacy (§3 borderline). Lean delegate for safety-critical.
- Does `eir-nursing` (first-touch, Q4 latency) deserve a latency-class agent like
  `eir-emergency`, or is it a skill + a latency hint on `eir-clinical`?
- Versioning skills (they're clinical content) — who curates/approves edits?
  (Ties to the Mimir curator / Living-Evidence track.)
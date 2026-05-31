# Design — Knowledge & Tool Layer (PrimeKG / ICD-10 / SNOMED → Skills)

- **Status:** Draft / Proposed
- **Date:** 2026-05-22
- **Decision:** [ADR-010](../../../Asgard/docs/decisions/ADR-010-agents-as-boundaries-skills-as-expertise.md)
- **Companions:** `medical-agent-architecture.md`, `medical-agent-data-model.md`,
  `Bifrost/docs/design/skill-loader-runtime.md`
- **Grounded in shipped code:** `Mimir/ro-ai-bridge/src/routes/knowledge_primekg.rs`
  (8 graph routes), `icd10.rs`, the SNOMED↔ICD-10-TM (`snomed_icd10_map`) and
  SNOMED↔MONDO (`snomed_mondo_map`) crosswalks, `/api/v1/knowledge/search`.

This note connects the **knowledge assets we already have** to the
agent/skill model: what counts as a *tool*, how a *skill* declares the tools and
knowledge it needs, and how lay/Thai clinical text gets grounded in the graph.

---

## 1. Tool vs Skill vs Knowledge — the three layers

```
SKILL ("cardio-acs-workup")        ← reasoning module; DECLARES tools + knowledge_scope
  ├─ allowed_tools:   [primekg_disease_relations, clinical_kb_search, pubmed_search]
  └─ knowledge_scope: { collection: clinical-wisdom, filter:{specialty: cardiology} }
        │ calls                                  │ retrieves
        ▼                                        ▼
TOOL (executable endpoint)                  KNOWLEDGE (data the tool reads)
  primekg_disease_relations  ──reads──▶  PrimeKG graph (Neo4j) + SNOMED/MONDO/ICD-10 maps
  clinical_kb_search         ──reads──▶  clinical-wisdom (Qdrant)
```

- A **tool** is an executable contract in the Hermodr/Mimir catalog.
- A **skill** declares *which* tools (⊆ host agent ceiling) and *which* knowledge
  scope it uses — it does not contain tools.
- The **agent ceiling** is the hard cap on tools (ADR-010). Skills narrow within
  it; the loader enforces `ceiling ∩ (base ∪ ⋃ skill.allowed_tools)`.

---

## 2. Tool catalog (what exists today)

Placement follows the Hybrid Tool Placement rule (stateful → Mimir in-process;
stateless/external → Hermodr). The **8 PrimeKG graph routes** we just shipped:

| Tool name | Endpoint (`/api/v1/knowledge/primekg/…`) | Param style | Home | Purpose |
|-----------|------------------------------------------|-------------|------|---------|
| `primekg_entity` | `/entity` | `name`, `entity_type` | Mimir | Exact name/type lookup |
| `primekg_resolve` | `/resolve` | `text` | Mimir | **text → SNOMED → MONDO → PrimeKG node** (lay/Thai friendly) |
| `primekg_disease_relations` | `/disease_relations` | `query` | Mimir | One-shot: extract disease → seed node → real edges. **Bypass-path friendly** |
| `primekg_neighbors` | `/neighbors` | `entity_index` (parametric) | Mimir | Multi-hop expand |
| `primekg_drug_interactions` | `/drug_interactions` | `drug_index` | Mimir | DRUG_DRUG edges (no native severity) |
| `primekg_disease_drugs` | `/disease_drugs` | `disease_index` | Mimir | INDICATION/CTRA/OFFLABEL |
| `primekg_symptom_to_disease` | `/symptom_to_disease` | `phenotype_names[]` | Mimir | Reverse phenotype |
| `primekg_path` | `/path` | `from_index`, `to_index` | Mimir | Shortest path(s) |

Plus existing: `icd10_lookup` / `icd10_resolve` (ICD-10-TM bilingual,
deterministic), `clinical_kb_search`, `vector_search`, `graph_search`,
`pubmed_search`, `drug_interaction_check` (Hermodr).

### 2.1 Param style matters for local models (bypass path)
The Bifrost heimdall/Gemini **bypass path auto-calls only tools that expose a
`query` param** (it can't drive parametric tools that need an `entity_index`).
So:

- **`primekg_resolve` (`text`)** and **`primekg_disease_relations` (`query`)** are
  the **bypass-friendly grounding tools** — a local-model agent can call them
  directly with free text.
- The parametric tools (`neighbors`, `drug_interactions`, `disease_drugs`, `path`)
  are **second-hop**: reached after a resolve/relations call yields an
  `entity_index`. Skills that need them must be on an agent using **native
  tool-calling**, or chained server-side. Note this in each skill's metadata.

---

## 3. The entity-resolution chain is a cross-cutting tool, not a specialty

`text → SNOMED → MONDO → PrimeKG` (`primekg_resolve`) and `text → SNOMED →
ICD-10-TM` (`icd10_resolve`) are **shared grounding primitives** every clinical
skill leans on — they belong to the **base tool set of `eir-clinical`**, not to
any one specialty skill. Properties to preserve as we wire them in:

- **Thai input** is normalized to a canonical English term first (resident
  gemma-4-26b, falling back to the ICD-10-TM bilingual dictionary). This is
  **license-safe** (SNOMED Affiliate clause 2.4): we translate a *lay term* to a
  *general English name*; we never translate SNOMED content.
- **Crosswalk licenses:** SNOMED = Affiliate (restricted); ICD-10-TM = MoPH;
  **MONDO = CC-BY-4.0**. The shared-KB catalog surfaces these (already wired in
  `shared_knowledge.rs`). Skills that ship to a customer box must respect the
  SNOMED affiliate boundary — this layer keeps SNOMED as an internal resolution
  step, not exported content.
- **Graceful deg: ** all PrimeKG tools return `503 neo4j_disabled` when the graph
  is off; resolvers fall back name→PrimeKG. Skills must tolerate empty grounding.

> ✅ **RESOLVED (2026-05-22, mimir-api v2.3.43):** the earlier SQL-injection
> finding in `primekg_resolve` is fixed — Step 0 (Thai fallback) and Step 1
> (SNOMED MATCH) now use `.bind()` + an `escape_like()` helper, verified
> injection-safe. **`primekg_resolve` and `primekg_disease_relations` are now
> safe to call with untrusted text.** (Original finding: string-formatted queries
> with only single-quote escaping, unlike the already-parameterized `icd10.rs`.)

---

## 4. `knowledge_scope` → retrieval

A skill's `knowledge_scope = { collection, filter }` parameterizes the existing
`manual_context` builders (vector/graph/tree search) so retrieval is scoped to
the active skill's domain.

| Knowledge asset | Store | Used via | Scope/filter key |
|-----------------|-------|----------|------------------|
| PrimeKG graph | Neo4j | `primekg_*` tools | entity type / relation |
| clinical-wisdom | Qdrant | `clinical_kb_search` / `vector_search` | `specialty` payload tag |
| ICD-10-TM, SNOMED↔ICD-10/MONDO | MariaDB | `icd10_*`, resolvers | terminology lookup |
| PubMed subset | (Hermodr/ext) | `pubmed_search` | — |

**Prerequisite (open):** `knowledge_scope.filter:{specialty: …}` assumes
clinical-wisdom chunks carry a `specialty` payload tag. They may not yet — a
**backfill** is needed (tag existing chunks, or derive specialty from matched
PrimeKG entity type at query time). Decide before phase-3 of the loader rollout.

---

## 5. Mapping tools to the 5 boundary agents

Each cell is that agent's **tool ceiling** (`agent_configs.tools`) — the enforced
maximum; the loader narrows per turn from active skills.

| Agent | Tool ceiling (highlights) |
|-------|---------------------------|
| **eir-clinical** | `primekg_resolve`, `primekg_disease_relations`, `clinical_kb_search`, `vector_search`, `graph_search`, `icd10_resolve`, `pubmed_search` (+ parametric `primekg_neighbors/disease_drugs/path` for native tool-calling skills) |
| **eir-pharmacy** | clinical ceiling + `primekg_drug_interactions`, `drug_interaction_check`, `dosage_calculator`, `formulary_lookup` |
| **eir-pediatrics** | clinical ceiling + `dosage_calculator (pediatric)` (the *enforced* boundary) |
| **eir-psychiatry** | clinical ceiling + `drug_interaction_check`, `clinical_calculator (PHQ-9/GAD-7)`; safety floor |
| **eir-emergency** | `clinical_kb_search`, `read_fhir`, `clinical_calculator`, `triage_score` (latency class) |

(`eir-forensic` deferred — see ADR-010 / arch §3.)

A cardiology *skill* on `eir-clinical` therefore grounds via `primekg_resolve` →
`primekg_disease_relations` (+ `clinical_kb_search` scoped to cardiology) — all
within the `eir-clinical` ceiling. It can never reach `dosage_calculator`,
because that lives only on pharmacy/peds ceilings (ADR-010 narrow-only).

---

## 6. Tool contract conventions

- **Result envelope** (per `mimir-skills` README): every tool returns
  `{ tool_name, status, data, sources[], latency_ms }`. PrimeKG tools should
  converge to this (currently return `{items, count}`); wrap or migrate.
- **Honest capability flags:** e.g. `primekg_drug_interactions` returns
  `severity_filter_supported: false` (PrimeKG has no native DDI severity) — keep
  such flags so skills/agents don't over-trust a result.
- **Sources/citations** must flow through (PrimeKG entity ids, SNOMED FSN, ICD
  code) so the overseer can attach citations to the final answer.

---

## 7. Open questions

- clinical-wisdom `specialty` tagging — backfill now vs derive at query time (§4).
- Should the resolve chain auto-run for *every* clinical turn (grounding-by-default
  on `eir-clinical`), or only when a skill requests it? (Lean: default-on for
  `eir-clinical`, cheap with the bypass-friendly `query` tools.)
- Converging `{items,count}` → the standard ToolResult envelope: one-time
  migration vs adapter at the loader boundary.
- Which `primekg_*` tools are safe to expose to local-model agents given the
  param-style constraint (§2.1) — finalize the bypass-friendly subset.
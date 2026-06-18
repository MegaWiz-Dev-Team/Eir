# 🏥 Eir: Multi-Agent Architecture & Design Guidelines

This document defines the architecture for **Eir Agents** — the specialized
healthcare AI agents within the Asgard ecosystem. Updated 2026-05-17:
13 medical specialties + 6 allied health roles, **single-tenant per Mac mini
deployment** (per [ADR-009](../../Asgard/docs/decisions/ADR-009-single-tenant-mac-mini-deployment.md)),
**local-LLM-only model recommendations** (per `feedback_eir_agents_local_only`),
tool allowlists, and orchestration flows.

**Revision history:**
- 2026-05-22 — §3 (19-agent roster) and §4.5 (specialty router) **refined** by [ADR-010](../../Asgard/docs/decisions/ADR-010-agents-as-boundaries-skills-as-expertise.md): direction is now "Agents = boundaries (~6), Skills = expertise (retrieved)", not one cloned agent per specialty. See note below.
- 2026-05-17 — Section 2 rewritten for single-tenant deployment; Section 3 model recommendations updated to local-only (removed gemini for emergency/ent/nursing); Sprint 38 PoC note clarified.
- 2026-05-05 — Initial 19-agent design.

> [!WARNING]
> **§3 (the 19-agent roster) and §4.5 (the specialty router) are being superseded.**
> Per [ADR-010](../../Asgard/docs/decisions/ADR-010-agents-as-boundaries-skills-as-expertise.md)
> (driven by the Sprint 38f finding that persona-routing gave **+0.0pp HBp** on
> the general mix), specialties move to **retrieved skills** on **5 boundary
> agents** (`eir-clinical`, `eir-pharmacy`, `eir-pediatrics`, `eir-psychiatry`,
> `eir-emergency`; `eir-forensic` deferred — no platform RBAC to enforce its
> access restriction), with skill selection by embedding and a **deterministic
> agent-resolver** replacing the LLM router. The deployment model (§2),
> local-LLM-only rule (§3 banner), and the multidisciplinary-team concept (§1)
> are **unchanged**.
>
> Authoritative design now lives in:
> - `design/medical-agent-architecture.md` — boundary/skill model + roster + §4b resolver
> - `design/medical-agent-data-model.md` — schema + 19→5 migration
> - `design/knowledge-tool-layer.md` — PrimeKG/ICD-10 tools + `knowledge_scope`
> - `Bifrost/docs/design/skill-loader-runtime.md` — the runtime selection/loader
>
> Treat the per-specialty tables below as the **source roster to port into
> skills**, not as agents to build as-is.

> [!IMPORTANT]
> "Eir" exists in **two distinct senses** in Asgard. Both are intentional:
>
> 1. **Eir Gateway** (this repo, `Eir/`) — OpenEMR-based FHIR gateway. The
>    *system of record* for patient/encounter/medication data.
> 2. **Eir Agents** (this doc) — LLM-driven specialist agents that *consume*
>    Eir Gateway data + Mimir RAG knowledge to answer clinical questions.
>    Live in Mimir's `agent_configs` table; orchestrated by Bifrost.
>
> Throughout this doc, "Eir" without qualifier = the Gateway data plane;
> "Eir Agents" = the LLM specialist layer.

---

## 1. Core Concept

In a real-world clinical setting, patient care is delivered by a **multidisciplinary
team**: an Internist diagnoses, a Pharmacist checks drug interactions, a
Nurse executes the care plan, a Dietitian builds the nutrition plan. Eir
Agents replicate this model — each agent is a focused specialist with its
own system prompt, allowlisted tools, and knowledge slice (PrimeKG +
clinical-wisdom + PubMed-abstracts subset).

The orchestrator (**Bifrost**) routes each user question to the most
appropriate agent (or fans out to multiple), synthesizes responses, and
attaches confidence + citations.

```
┌──────────────────────────────────────────────────────────────────┐
│  Physician asks: "65y M, T2DM, HbA1c 9.2, eGFR 38, on metformin │
│   1g BID + lisinopril 20mg. Next step?"                          │
└────────────────────────────────┬─────────────────────────────────┘
                                 ▼
              ┌───────────────────────────────────┐
              │  ⚡ Bifrost (Orchestrator)         │
              │   Specialty Router classifies →   │
              │   Internal Medicine + Pharmacy +  │
              │   Nephrology (eGFR <45 → CKD path)│
              └─────┬─────────────┬──────────────┬┘
                    ▼             ▼              ▼
        ┌───────────────┐ ┌───────────────┐ ┌──────────────┐
        │ 🩺 Internal   │ │ 💊 Pharmacy   │ │ 🫘 Nephrology│
        │   Medicine    │ │   Agent        │ │   Agent      │
        │ "DM ladder +  │ │ "Metformin     │ │ "eGFR 38 →   │
        │  add SGLT2i"  │ │  contraind. at │ │  hold metform.│
        │               │ │  eGFR<30, 50%  │ │  prefer DPP-4│
        │               │ │  reduce <45"   │ │  or SGLT2i"  │
        └───────┬───────┘ └───────┬───────┘ └──────┬───────┘
                └─────────┬───────┴────────────────┘
                          ▼
              ┌───────────────────────────────────┐
              │  ⚡ Bifrost: Synthesis             │
              │   Conflict resolution +           │
              │   Confidence aggregation +        │
              │   Citation merge                  │
              └─────────────────┬─────────────────┘
                                ▼
              ┌───────────────────────────────────┐
              │  Final Answer + Confidence + Refs │
              │   "Hold metformin (eGFR 38).      │
              │    Add empagliflozin 10mg QD      │
              │    (SGLT2i — renoprotective +     │
              │    glycemic). Confidence: 0.91.   │
              │    Refs: KDIGO 2024, ADA 2025."   │
              └───────────────────────────────────┘
```

---

## 2. Deployment Model — Single-Tenant Per Mac Mini

> **Goal:** each customer (hospital) receives a dedicated on-prem Mac mini
> running the full Asgard stack. The 19 Eir agents live on that one box.
> No cross-customer infrastructure. Per [ADR-009](../../Asgard/docs/decisions/ADR-009-single-tenant-mac-mini-deployment.md),
> Asgard is **NOT** a SaaS multi-tenant platform.

### 2.1 Tenancy scope

The 19 Eir agents live exclusively on **`asgard_medical`** tenant boxes
(hospitals, clinics). They do **NOT** deploy to `asgard_insurance` boxes.
Insurance has its own Underwriter consensus agents + a restricted 3-agent
read-only Eir subset (`eir-internal-medicine`, `eir-medtech`, `eir-pharmacy`)
for clinical interpretation of applicant medical history — not the full 19.

| What | Where |
|---|---|
| 19 Eir agents (`agent_configs` rows) | asgard_medical box only |
| Eir Gateway (OpenEMR FHIR data plane) | asgard_medical box only |
| Restricted 3-agent Eir subset (read-only) | asgard_insurance box |
| Underwriter consensus agents | asgard_insurance box only |

### 2.2 Three layers of isolation

| Layer | How isolation works | Strength |
|---|---|---|
| **Physical hardware** | Each customer has their own Mac mini. Patient data never leaves the box. | Strongest — no code-level multi-tenancy logic can match physical separation |
| **Tenant config** | `tenant_id ∈ {asgard_medical, asgard_insurance}` is a deployment configuration constant, NOT a runtime isolation primitive. Each box's `agent_configs` table contains rows for ONE tenant_id. | Belt-and-suspenders |
| **JWT auth** | Yggdrasil-issued JWT proves caller is authorized to use THIS box. It does NOT route between tenants (no cross-box calls). | Service-to-service auth, not multi-tenant routing |

### 2.3 No RBAC, no cross-box, no shared-cluster

The earlier (pre-2026-05-17) design described a SaaS multi-tenant cluster:
"one Asgard deployment serves N hospitals", per-tenant Qdrant filters,
atomic tenant onboarding clone. **That model is rejected.** The reality:

- 100 hospitals = 100 Mac minis = 100 independent deployments
- No code path joins data across customers
- No RBAC at the platform layer — single org per box, customer manages
  their internal user access via their own IDM (or via Zitadel local instance)
- Updates roll out per box via CI/runbook discipline, not central deploy
- Audit log (Tyr) is local per box; no central SIEM (until customer-fleet
  management becomes a thing, separate roadmap)

### 2.4 What IS shared, what IS NOT

| What | Shared across boxes? | Notes |
|---|---|---|
| **Agent templates** (system_prompt, tool list, model_id structure) | Yes — same template definitions ship with every Asgard release | Each box materializes them into its `agent_configs` via seed migration |
| **PrimeKG** (Harvard public KG) | Yes — every box loads the same public knowledge | One-time load per box; refresh on Harvard releases via runbook |
| **PubMed abstracts subset** | Yes — public reference corpus | Same as PrimeKG |
| **Clinical calculators** | Yes — formulas don't vary by customer | Stateless tools in Hermodr |
| **Patient data** | NO — never leaves the box | OpenEMR/Eir Gateway local DB only |
| **RefGraph** (document-derived entities) | NO — built from customer's own documents | Per-deployment Neo4j |
| **Hospital formulary / SOP / protocol overrides** | NO — customer-specific | Loaded into their Qdrant + injected into prompts at request time |
| **Audit log** | NO — local per box | LocalDbSink + optional Wazuh stub forward |

### 2.5 Onboarding sequence (per Mac mini)

```
1. Provision Mac mini (physical or remote install)
2. Run ./scripts/deploy-all.sh
   → Installs Bifrost / Heimdall / Mimir / Skuggi / Tyr / Hermodr / Syn
   → Starts MariaDB + Qdrant + Neo4j pods
3. Run tenant seed:
   mysql < Mimir/scripts/recover-asgard-medical-tenant.sql
   → INSERT 19 rows into agent_configs (tenant_id='asgard_medical')
   → CREATE Qdrant collections: chunks-asgard_medical, pages-asgard_medical
   → Load Eir Gateway (OpenEMR) schema
4. Load shared knowledge:
   - PrimeKG via Mimir/scripts/primekg_import.sh
   - ICD-10-TM via Mimir/scripts/icd10_tm_anamai_ingest.py
5. Customer-specific ingestion (ongoing):
   - Hospital uploads SOPs / formularies / protocols
   - Ingested into their LOCAL Qdrant collection only
6. Yggdrasil issues JWT for authorized users; JWT carries the tenant_id
   constant ('asgard_medical') for audit attribution
7. Every Bifrost call validates JWT and emits audit event;
   no cross-box routing because there is no cross-box anything
```

---

## 3. The 19 Eir Agents

Each agent has: an icon, a primary role, allowlisted tools (per Asgard
`MultiAgent_Architecture_Plan.md` §10 Security Sandbox Deny-by-default),
and a default model recommendation. Names follow the convention
**`eir-<specialty-slug>`** (snake-case, lowercase) for stable IDs across
tenants.

> **🚫 Model rule — LOCAL ONLY:** All 19 Eir agents must use **local LLM**.
> Cloud LLM (Gemini, OpenAI, Claude, etc.) is **BANNED** as the default
> reasoning model for any Eir agent — even for low-latency requirements
> like Emergency (≤2s p50). Use quantized variants (Q4/Q8 of gemma-4 via
> Heimdall MLX) to meet latency budgets locally.
> See [`feedback_eir_agents_local_only`](../../) memory for rationale
> (data sovereignty, latency consistency, offline operation, cost
> predictability).
>
> Acceptable defaults:
> - `gemma-4-26b` — champion, used by most agents
> - `gemma-4-26b Q4` or `Q8` — for low-latency agents (emergency, nursing)
> - `medgemma-27b-text` — for safety-critical agents (pediatrics, psychiatry)
> - `typhoon-1.5-3b` — Thai-tuned (OCR champion; also viable for Thai-heavy
>   reasoning if benchmarked)
>
> Cloud LLM remains available via non-Eir tools (e.g., `pubmed_search` if
> backed by a remote API) — but never as the **reasoning model** of an
> Eir agent.

### 3.1 Medical Specialist Agents (13)

| # | Agent | Slug | Primary Role | Allowlisted Tools | Default Model |
|:-:|:--|:--|:--|:--|:--|
| 1 | 🩺 **Internal Medicine** | `eir-internal-medicine` | Diagnoses & treats internal diseases (T2DM, HTN, CKD, COPD). The "general physician" fallback when no narrower specialty is identified. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search`, `clinical_calculator` (CHADS2/MELD/eGFR) | gemma-4-26b (champion) |
| 2 | 🪒 **Surgery** | `eir-surgery` | Surgical planning, pre/post-op care, complication management, surgical-site infection screening. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 3 | 👶 **Pediatrics** | `eir-pediatrics` | Child development, age/weight-based dosing, childhood diseases, vaccination schedules. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `dosage_calculator` (pediatric-aware), `pubmed_search` | medgemma-27b-text (Sprint 43 candidate) |
| 4 | 👁️ **Ophthalmology** | `eir-ophthalmology` | Eye diseases, vision abnormalities, diabetic retinopathy screening guidance. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 5 | 🦴 **Orthopedics** | `eir-orthopedics` | Bone, joint, muscle injuries, fracture management, rehab referrals. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 6 | 🚑 **Emergency Medicine** | `eir-emergency` | Triage, CPR/ALS guidance, critical-condition assessment. **Strict latency budget** (≤2s p50). | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (Wells, NEXUS, GCS), `triage_score` | gemma-4-26b Q4 (Heimdall MLX quantized — local, low-latency) |
| 7 | 🤰 **OB-GYN** | `eir-ob-gyn` | Women's health, pregnancy management, delivery, perinatal pharmacology. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check` (pregnancy categories), `pubmed_search` | gemma-4-26b |
| 8 | 💉 **Anesthesiology** | `eir-anesthesia` | Anesthesia dosing, perioperative pain management, ASA classification. | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (ASA, Mallampati), `dosage_calculator` | gemma-4-26b |
| 9 | 👃 **ENT** | `eir-ent` | Ear, Nose, Throat conditions, upper-respiratory tract issues. **Sprint 38 specialty** (already deployed PoC). | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 10 | 🩺 **Urology** | `eir-urology` | Urinary tract & male reproductive system diseases, renal stones. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 11 | ⚖️ **Forensic Medicine** | `eir-forensic` | Autopsy analysis, forensic-evidence reporting, cause-of-death documentation. **Restricted access** (forensic team only). | `search_primekg`, `read_fhir` (read-only), `pubmed_search` | gemma-4-26b |
| 12 | 🧠 **Psychiatry** | `eir-psychiatry` | Mental-health screening (PHQ-9, GAD-7), therapy framing, psychotropic medication management. **Safety floor**: hard refuse on suicide-method requests. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check`, `clinical_calculator` (PHQ-9, GAD-7) | medgemma-27b-text (Sprint 43 candidate; Google red-team safety) |
| 13 | 📷 **Radiology** | `eir-radiology` | Interpretation framing for X-ray/CT/MRI, ordering guidance, ALARA dose review. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search`, `image_metadata_lookup` | gemma-4-26b (text only — image multimodal in Sprint 45+) |

### 3.2 Allied Health & Support Agents (6)

| # | Agent | Slug | Primary Role | Allowlisted Tools | Default Model |
|:-:|:--|:--|:--|:--|:--|
| 14 | 💊 **Pharmacy** | `eir-pharmacy` | Drug-Drug Interactions (DDI) screen, dosage calculation, ADR monitoring, formulary check. **Always invoked when prescription action proposed.** | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check`, `dosage_calculator`, `formulary_lookup` | gemma-4-26b |
| 15 | 🔬 **Medical Technology** | `eir-medtech` | Lab result interpretation, trend analysis, microbiology/antibiograms. | `search_clinical_kb`, `read_fhir` (labs), `lab_reference_range`, `antibiogram_lookup` | gemma-4-26b |
| 16 | 👩‍⚕️ **Nursing** | `eir-nursing` | Triage, vitals monitoring, care-plan tracking, patient education materials. **First-touch agent** for most flows. | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (triage scores), `patient_education_lookup` | gemma-4-26b Q4 (Heimdall MLX quantized — local, low-latency triage) |
| 17 | 🤸 **Physical Therapy** | `eir-pt` | Rehab program design, mobility tracking, post-surgery PT scheduling. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 18 | 🥗 **Dietitian** | `eir-dietitian` | Disease-specific nutritional planning, drug-food interaction screen. | `search_clinical_kb`, `read_fhir`, `drug_food_interaction`, `nutrition_calculator` | gemma-4-26b |
| 19 | 🧑‍🤝‍🧑 **Social Worker / Psychology** | `eir-social-work` | Mental-health support pathways, socio-economic patient assessment, community-resource navigation. | `search_clinical_kb`, `read_fhir`, `community_resource_lookup` (per-tenant) | gemma-4-26b |

> **Tool naming convention:** Stateful tools (PrimeKG, Qdrant, FHIR) live
> in Mimir; stateless / external-API tools (PubMed, drug interactions,
> calculators) live in Hermodr per Asgard
> [`MultiAgent_Architecture_Plan.md` §11 Hybrid Tool Placement](../../Asgard/docs/roadmap/MultiAgent_Architecture_Plan.md#11-communication-protocols-mcp--a2a).
> Eir Agents see one flat catalog regardless of where each tool runs.

---

## 4. Multi-Agent Orchestration Flow

### 4.1 Standard outpatient encounter

```
1. 👩‍⚕️ Nursing Agent       → triage, capture vitals, route by chief complaint
2. 🩺 Internal Medicine    → differential diagnosis, order labs/imaging
3. 🔬 Medical Technology   → interpret lab results, flag abnormals
4. 🩺 Internal Medicine    → finalize diagnosis, propose Rx
5. 💊 Pharmacy             → DDI screen + dose-vs-eGFR check (always)
6. 🥗 Dietitian (cond.)    → if diabetic/CKD/HTN → nutrition plan
7. 👩‍⚕️ Nursing Agent       → patient education + follow-up scheduling
```

### 4.2 Surgical flow

```
1. 🩺 Internal Medicine    → pre-op clearance (cards/pulm risk)
2. 💉 Anesthesiology       → ASA classification, anesthetic plan
3. 🪒 Surgery              → procedure plan, complication risk
4. 💊 Pharmacy             → peri-op medication management (anticoag hold)
5. 👩‍⚕️ Nursing Agent       → pre-op checklist, post-op care plan
6. 🤸 Physical Therapy     → rehab schedule (if applicable)
```

### 4.3 Emergency triage

```
1. 🚑 Emergency Medicine   → ESI triage, immediate-risk assessment
   ├ if cardiac → 🩺 Internal Medicine + 💊 Pharmacy
   ├ if trauma  → 🪒 Surgery + 🦴 Orthopedics
   └ if mental  → 🧠 Psychiatry + 🧑‍🤝‍🧑 Social Worker
```

### 4.4 Pediatric encounter

```
1. 👩‍⚕️ Nursing Agent       → triage with pediatric vitals (age-specific)
2. 👶 Pediatrics           → age/weight-based assessment + dosing
3. 💊 Pharmacy             → pediatric DDI + dose-band check
4. 🥗 Dietitian (cond.)    → growth-chart based nutrition (if abnormal BMI)
```

### 4.5 Routing decision rules (Bifrost specialty router)

| Trigger | Route to |
|---|---|
| Chief complaint contains pediatric markers (age <18, "child", "infant") | `eir-pediatrics` first |
| Lab order or lab-result interpretation | `eir-medtech` |
| Any prescription action (`MedicationRequest` create/update) | `eir-pharmacy` always |
| Mental-health screening keywords (PHQ, GAD, suicidal ideation) | `eir-psychiatry` first |
| Surgical-site infection / post-op complication | `eir-surgery` + `eir-pharmacy` |
| Imaging order or imaging report attached | `eir-radiology` |
| Mental-health + socio-economic complexity | `eir-psychiatry` + `eir-social-work` |
| No specialty markers (low confidence) | `eir-internal-medicine` (generic fallback) |

---

## 5. Implementation Roadmap

### 5.1 Current state (2026-05-05)

- ✅ **Sprint 38 PoC LIVE** — 5 agents deployed for `asgard_medical` tenant:
  `eir` (generic), `eir-cardio`, `eir-sleep`, `eir-ent`, `eir-pediatrics`,
  plus `eir-router`. 5/5 routing accuracy on hand-picked questions.
  > **Note 2026-05-17:** `eir-cardio` and `eir-sleep` are PoC names from
  > Sprint 38 and are **NOT** in the official 19-agent set. Their scope is
  > absorbed by `eir-internal-medicine` (cardio is a subset; sleep is a
  > subset of pulmonology/IM). Migration from PoC names to the production
  > 19-set is part of P1 rollout (Sprint 43+). The generic `eir` will be
  > renamed to `eir-internal-medicine` during the migration.
- 🟠 **Sprint 38f router validation** — A/B router-vs-monolithic
  `Δ = +0.0pp HBp` on hb-pro general mix → router ROI is on
  specialty-tagged subsets, not general mix. **Aborted 28-specialty
  expansion**; this 19-agent set is the right scope.

### 5.2 Roll-out plan

| Phase | What | When | Status |
|---|---|---|---|
| **P1** | Template the 19 agents in `agent_configs` (per-tenant clone) | Sprint 43+ | 📋 |
| **P2** | Pre-flight safety screen (Sprint 43 B-61) gates promotion of any new model into any agent | Sprint 43 | 🟡 in flight |
| **P3** | Per-Mac-mini install runbook: seed 19 agents into `agent_configs` (single tenant_id='asgard_medical'), create local Qdrant collections, install Eir Gateway/OpenEMR. See [ADR-009](../../Asgard/docs/decisions/ADR-009-single-tenant-mac-mini-deployment.md); NOT a multi-tenant cloning wizard | Sprint 43+ | 📋 |
| **P4** | Per-specialty HBp% breakdown (Sprint 38f B-55) so each agent has its own scoreboard | Sprint 40f | 📋 |
| **P5** | Per-agent tool allowlist enforced server-side (deny-by-default per Asgard §10) | Sprint 38f | 🟠 partial |
| **P6** | Image-multimodal Eir Radiology (medgemma-27b vision) | Sprint 45+ | ❄️ future |

### 5.3 Acceptance criteria (per agent)

Before any Eir Agent can ship to a real customer (Mac mini deployment):
- ✅ Pre-flight safety screen passes 20/20 on agent's default model
- ✅ Tool allowlist enforced (call to non-allowlisted tool → 403)
- ✅ HBp% baseline ≥ generic Eir on agent's specialty subset (no regression)
- ✅ Latency p50 within budget (Emergency: ≤2s, Nursing triage: ≤3s, others: ≤8s) **on local LLM** (Heimdall MLX quantized variants are allowed for low-latency)
- ✅ Default model is **local** (no gemini/openai/claude) — see Model rule in Section 3
- ✅ Audit trail captures `(tenant_id, agent_name, tool_calls, model_id, citations)` to LocalDbSink + (optional) Wazuh stub

---

## 6. Cross-references

- **Asgard Architecture (umbrella):** [`Asgard/docs/roadmap/MultiAgent_Architecture_Plan.md`](../../Asgard/docs/roadmap/MultiAgent_Architecture_Plan.md) — agent registry, security sandbox, MCP/A2A protocols, hybrid tool placement
- **Mimir Sprint plan:** [`Mimir/docs/03_implementation_plans/03_14_Local_LLM_Optimization_Sprints.md`](../../Mimir/docs/03_implementation_plans/03_14_Local_LLM_Optimization_Sprints.md) — Sprint 38 router PoC, Sprint 38f validation, Sprint 43 model-alternative challenges
- **HealthBench-Pro baseline:** [`Mimir/docs/04_evaluation_and_testing/04_03_HealthBench_Pro_Baseline_2026-05-04.md`](../../Mimir/docs/04_evaluation_and_testing/04_03_HealthBench_Pro_Baseline_2026-05-04.md) — canonical scoreboard for Eir Agents
- **Asgard Sprint planning:** [`Asgard/docs/sprint-planning.md`](../../Asgard/docs/sprint-planning.md) — Week 3 P2 includes "Eir Multi-Agent Architecture" task

## 7. Open questions / decisions parking lot

- **Agent voice / persona consistency** — should all 19 agents share a base persona (e.g. "I am a clinical assistant supporting your decision; final responsibility remains with the licensed clinician"), or per-specialty tone? *Tentative: shared base + per-specialty override.*
- **Cross-agent disagreement protocol** — when Internal Medicine recommends drug X but Pharmacy flags an interaction, who arbitrates? *Tentative: Bifrost surfaces conflict, Pharmacy's safety flag is sticky (cannot be overridden by Internal Medicine within the same turn).*
- **Forensic Medicine deployment** — should `eir-forensic` deploy to all `asgard_medical` boxes (with role-based tool restriction limiting access to forensic team only), or only to dedicated forensic-clinic Mac mini deployments? Note: per [ADR-009](../../Asgard/docs/decisions/ADR-009-single-tenant-mac-mini-deployment.md), "separate tenant" no longer means SaaS isolation — it would mean a separate physical Mac mini. *Open — pending legal review + forensic customer signal.*
- **Multilingual support** — Thai-language prompts vs English-with-Thai-output. *Sprint 46+ — depends on per-language HBp baselines.*

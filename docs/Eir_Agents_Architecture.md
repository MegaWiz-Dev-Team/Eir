# 🏥 Eir: Multi-Agent Architecture & Design Guidelines

This document defines the architecture for **Eir Agents** — the specialized
healthcare AI agents within the Asgard ecosystem. Updated 2026-05-05 with
13 medical specialties + 6 allied health roles, multi-tenant tenancy model,
tool allowlists, and orchestration flows.

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

## 2. Multi-Tenant Design Principles

> **Goal:** one Asgard deployment serves N hospitals/clinics. Agent logic
> shared (cost-efficient); patient data + tenant-specific rules strictly
> isolated (PDPA + HIPAA).

### 2.1 Three layers of separation

| Layer | What's shared | What's isolated per tenant | Enforced by |
|---|---|---|---|
| **Agent logic** | system prompt template, tool allowlist, model_id, default temp/top_k | per-tenant `agent_configs.tenant_id` row (template clone) | `Mimir.agent_configs` PK = `(tenant_id, name)` |
| **Knowledge / RAG** | PrimeKG topology (one ontology), generic PubMed abstracts | tenant-specific docs, hospital formularies, internal SOPs | Qdrant collection per tenant + `metadata.tenant_id` filter |
| **Patient data** | nothing | encounters, FHIR resources, chat history, audit trail | OpenEMR per-tenant DB schema; Yggdrasil JWT carries `tenant_id` |

### 2.2 Why this works

- **One agent template, N tenants** = each new hospital onboarded via
  one SQL transaction (clone template row, set `tenant_id`). No code
  deploy. See `Mimir/admin/onboard-tenant` (Sprint 38f B-53).
- **Dynamic guidelines injection at runtime** — Bifrost reads
  `tenant_settings.formulary` + `tenant_settings.protocol_overrides`
  and prepends them to the agent's system prompt for that request only.
  No tenant secrets ever bake into shared model weights.
- **Data sovereignty for enterprise clients** — Tier-1 deployments can
  pin Qdrant + MariaDB + OpenEMR to a customer-controlled namespace
  (Asgard Phase-1 "Data Sovereignty" track) while still calling the
  shared model gateway (Heimdall).

### 2.3 Tenant onboarding sequence

```
1. Admin creates tenant in Yggdrasil (gets tenant_id UUID)
2. POST /admin/onboard-tenant (Mimir)
   → INSERT 19 rows into agent_configs (1 per Eir Agent template)
   → CREATE Qdrant collection "tenant_<id>_knowledge"
   → CREATE OpenEMR schema "eir_<id>"
3. Tenant uploads internal SOPs / formularies → ingested into their
   Qdrant collection only
4. Tenant users log in (Zitadel), JWT carries tenant_id
5. Every Bifrost call filters Qdrant + agent_config by JWT.tenant_id
```

---

## 3. The 19 Eir Agents

Each agent has: an icon, a primary role, allowlisted tools (per Asgard
`MultiAgent_Architecture_Plan.md` §10 Security Sandbox Deny-by-default),
and a default model recommendation. Names follow the convention
**`eir-<specialty-slug>`** (snake-case, lowercase) for stable IDs across
tenants.

### 3.1 Medical Specialist Agents (13)

| # | Agent | Slug | Primary Role | Allowlisted Tools | Default Model |
|:-:|:--|:--|:--|:--|:--|
| 1 | 🩺 **Internal Medicine** | `eir-internal-medicine` | Diagnoses & treats internal diseases (T2DM, HTN, CKD, COPD). The "general physician" fallback when no narrower specialty is identified. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search`, `clinical_calculator` (CHADS2/MELD/eGFR) | gemma-4-26b (champion) |
| 2 | 🪒 **Surgery** | `eir-surgery` | Surgical planning, pre/post-op care, complication management, surgical-site infection screening. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 3 | 👶 **Pediatrics** | `eir-pediatrics` | Child development, age/weight-based dosing, childhood diseases, vaccination schedules. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `dosage_calculator` (pediatric-aware), `pubmed_search` | medgemma-27b-text (Sprint 43 candidate) |
| 4 | 👁️ **Ophthalmology** | `eir-ophthalmology` | Eye diseases, vision abnormalities, diabetic retinopathy screening guidance. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 5 | 🦴 **Orthopedics** | `eir-orthopedics` | Bone, joint, muscle injuries, fracture management, rehab referrals. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 6 | 🚑 **Emergency Medicine** | `eir-emergency` | Triage, CPR/ALS guidance, critical-condition assessment. **Strict latency budget** (≤2s p50). | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (Wells, NEXUS, GCS), `triage_score` | gemini-3.1-flash-lite (cloud, low-latency) |
| 7 | 🤰 **OB-GYN** | `eir-ob-gyn` | Women's health, pregnancy management, delivery, perinatal pharmacology. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check` (pregnancy categories), `pubmed_search` | gemma-4-26b |
| 8 | 💉 **Anesthesiology** | `eir-anesthesia` | Anesthesia dosing, perioperative pain management, ASA classification. | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (ASA, Mallampati), `dosage_calculator` | gemma-4-26b |
| 9 | 👃 **ENT** | `eir-ent` | Ear, Nose, Throat conditions, upper-respiratory tract issues. **Sprint 38 specialty** (already deployed PoC). | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemini-3.1-flash-lite |
| 10 | 🩺 **Urology** | `eir-urology` | Urinary tract & male reproductive system diseases, renal stones. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search` | gemma-4-26b |
| 11 | ⚖️ **Forensic Medicine** | `eir-forensic` | Autopsy analysis, forensic-evidence reporting, cause-of-death documentation. **Restricted access** (forensic team only). | `search_primekg`, `read_fhir` (read-only), `pubmed_search` | gemma-4-26b |
| 12 | 🧠 **Psychiatry** | `eir-psychiatry` | Mental-health screening (PHQ-9, GAD-7), therapy framing, psychotropic medication management. **Safety floor**: hard refuse on suicide-method requests. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check`, `clinical_calculator` (PHQ-9, GAD-7) | medgemma-27b-text (Sprint 43 candidate; Google red-team safety) |
| 13 | 📷 **Radiology** | `eir-radiology` | Interpretation framing for X-ray/CT/MRI, ordering guidance, ALARA dose review. | `search_primekg`, `search_clinical_kb`, `read_fhir`, `pubmed_search`, `image_metadata_lookup` | gemma-4-26b (text only — image multimodal in Sprint 45+) |

### 3.2 Allied Health & Support Agents (6)

| # | Agent | Slug | Primary Role | Allowlisted Tools | Default Model |
|:-:|:--|:--|:--|:--|:--|
| 14 | 💊 **Pharmacy** | `eir-pharmacy` | Drug-Drug Interactions (DDI) screen, dosage calculation, ADR monitoring, formulary check. **Always invoked when prescription action proposed.** | `search_primekg`, `search_clinical_kb`, `read_fhir`, `drug_interaction_check`, `dosage_calculator`, `formulary_lookup` | gemma-4-26b |
| 15 | 🔬 **Medical Technology** | `eir-medtech` | Lab result interpretation, trend analysis, microbiology/antibiograms. | `search_clinical_kb`, `read_fhir` (labs), `lab_reference_range`, `antibiogram_lookup` | gemma-4-26b |
| 16 | 👩‍⚕️ **Nursing** | `eir-nursing` | Triage, vitals monitoring, care-plan tracking, patient education materials. **First-touch agent** for most flows. | `search_clinical_kb`, `read_fhir`, `clinical_calculator` (triage scores), `patient_education_lookup` | gemini-3.1-flash-lite (low-latency triage) |
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
- 🟠 **Sprint 38f router validation** — A/B router-vs-monolithic
  `Δ = +0.0pp HBp` on hb-pro general mix → router ROI is on
  specialty-tagged subsets, not general mix. **Aborted 28-specialty
  expansion**; this 19-agent set is the right scope.

### 5.2 Roll-out plan

| Phase | What | When | Status |
|---|---|---|---|
| **P1** | Template the 19 agents in `agent_configs` (per-tenant clone) | Sprint 43+ | 📋 |
| **P2** | Pre-flight safety screen (Sprint 43 B-61) gates promotion of any new model into any agent | Sprint 43 | 🟡 in flight |
| **P3** | Tenant onboarding wizard `POST /admin/onboard-tenant` (atomic 19-agent clone + Qdrant + OpenEMR schema) | Sprint 38f B-53 | 📋 |
| **P4** | Per-specialty HBp% breakdown (Sprint 38f B-55) so each agent has its own scoreboard | Sprint 40f | 📋 |
| **P5** | Per-agent tool allowlist enforced server-side (deny-by-default per Asgard §10) | Sprint 38f | 🟠 partial |
| **P6** | Image-multimodal Eir Radiology (medgemma-27b vision) | Sprint 45+ | ❄️ future |

### 5.3 Acceptance criteria (per agent)

Before any Eir Agent can ship to a real tenant:
- ✅ Pre-flight safety screen passes 20/20 on agent's default model
- ✅ Tool allowlist enforced (call to non-allowlisted tool → 403)
- ✅ HBp% baseline ≥ generic Eir on agent's specialty subset (no regression)
- ✅ Latency p50 within budget (Emergency: ≤2s, Nursing triage: ≤3s, others: ≤8s)
- ✅ Audit trail captures `(tenant_id, agent_name, tool_calls, model_id, citations)`

---

## 6. Cross-references

- **Asgard Architecture (umbrella):** [`Asgard/docs/roadmap/MultiAgent_Architecture_Plan.md`](../../Asgard/docs/roadmap/MultiAgent_Architecture_Plan.md) — agent registry, security sandbox, MCP/A2A protocols, hybrid tool placement
- **Mimir Sprint plan:** [`Mimir/docs/03_implementation_plans/03_14_Local_LLM_Optimization_Sprints.md`](../../Mimir/docs/03_implementation_plans/03_14_Local_LLM_Optimization_Sprints.md) — Sprint 38 router PoC, Sprint 38f validation, Sprint 43 model-alternative challenges
- **HealthBench-Pro baseline:** [`Mimir/docs/04_evaluation_and_testing/04_03_HealthBench_Pro_Baseline_2026-05-04.md`](../../Mimir/docs/04_evaluation_and_testing/04_03_HealthBench_Pro_Baseline_2026-05-04.md) — canonical scoreboard for Eir Agents
- **Asgard Sprint planning:** [`Asgard/docs/sprint-planning.md`](../../Asgard/docs/sprint-planning.md) — Week 3 P2 includes "Eir Multi-Agent Architecture" task

## 7. Open questions / decisions parking lot

- **Agent voice / persona consistency** — should all 19 agents share a base persona (e.g. "I am a clinical assistant supporting your decision; final responsibility remains with the licensed clinician"), or per-specialty tone? *Tentative: shared base + per-specialty override.*
- **Cross-agent disagreement protocol** — when Internal Medicine recommends drug X but Pharmacy flags an interaction, who arbitrates? *Tentative: Bifrost surfaces conflict, Pharmacy's safety flag is sticky (cannot be overridden by Internal Medicine within the same turn).*
- **Forensic Medicine isolation** — should `eir-forensic` live in a separate tenant entirely (forensic-only data) or share with the hospital tenant under stricter ACL? *Open — pending legal review.*
- **Multilingual support** — Thai-language prompts vs English-with-Thai-output. *Sprint 46+ — depends on per-language HBp baselines.*

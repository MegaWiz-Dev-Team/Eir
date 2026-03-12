# Release Notes — Eir Gateway

## v0.3.0 — Asgard Integration (2026-03-12)

> Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.

### ✨ New Features
- **Bifrost Agent Tools** — 7 tool endpoints (query_patient, search_patients, patient_summary, list_encounters, get_vitals, get_medications, clinical_summary) for Bifrost ReAct agent
- **Mimir Knowledge Sync** — POST /api/knowledge/sync for pushing clinical knowledge to Mimir RAG
- **A2A Protocol** — /.well-known/agent.json Agent Card + POST /a2a/tasks for agent-to-agent collaboration
- **CORS wildcard** for agent access

### 📊 Stats
- **47 tests**, 0 warnings
- 7 source modules

---

## v0.2.0 — FHIR & Performance (2026-03-11)

### ✨ New Features
- FHIR R4-aware proxy (/fhir/r4/*) with resource-type detection
- moka async in-memory cache (configurable TTL)
- governor rate limiting (GCRA algorithm, per-IP)
- Request header transformation (X-Forwarded-For, X-Tenant-Id, X-Request-Id)
- OpenAPI spec + Scalar UI (/api-docs)

### 📊 Stats
- **22 tests**, 0 warnings

---

## v0.1.0 — Foundation (2026-03-11)

### ✨ New Features
- Axum web server with full middleware stack (CORS → Audit → RateLimit → Auth → Transform → Cache)
- Reverse proxy to OpenEMR backend
- Bearer token authentication (public path allowlist)
- Structured JSON audit logging
- Health checks (/healthz, /readyz)

### 📊 Stats
- **10 tests**, 0 warnings

# Release Notes — Eir (API Gateway)

## v0.4.0 — Zitadel JWKS Auth (2026-03-15)

### 🔒 Security
- **JWKS-based JWT validation** — replaces static Bearer token
- `JwksCache` fetches + caches RS256 keys from Zitadel
- `ZitadelClaims` extracts `sub`, `org_id`, `roles` from tokens
- Fallback to static `AUTH_SECRET` when `ZITADEL_ISSUER` is empty
- New config: `ZITADEL_ISSUER`, `JWT_AUDIENCE`

### 📊 Stats
- **57 tests**, all passing (0.21s)
- Clippy clean (0 warnings)
- Sprint 4 complete (ISO 29110 PM-02-04)

---

## v0.3.0 — A2A Protocol + Agent Tools (2026-03-11)

### ✨ Features
- A2A protocol support (Google Agent-to-Agent)
- Agent tool endpoints (patient_search, fhir_query, clinical_summary)
- Mimir knowledge sync webhooks
- Chat interface endpoint

---

## v0.2.0 — FHIR Proxy + Caching + Rate Limiting (2026-03-11)

### ✨ Features
- FHIR R4-aware reverse proxy
- In-memory response caching (moka)
- Per-IP rate limiting (governor / GCRA)
- Request transformation (tenant headers, path rewrite)
- OpenAPI docs with Scalar UI

---

## v0.1.0 — Foundation (2026-03-11)

### ✨ Features
- Reverse proxy to OpenEMR
- Bearer token authentication
- Health check endpoints
- Structured audit logging
- CORS support

---

*Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.*

# PM-02-02: Sprint 2 Report — FHIR Proxy + Enhancement
**Sprint:** 2
**Period:** 2026-03-12
**Status:** ✅ Completed

---

## Scope of Work

| Deliverable | Status | File |
|:--|:--|:--|
| FHIR R4 proxy (/fhir/r4/*) | ✅ Done | `src/fhir.rs` |
| Response caching (moka) | ✅ Done | `src/cache.rs` |
| Rate limiting (governor/GCRA) | ✅ Done | `src/rate_limit.rs` |
| Request transformation | ✅ Done | `src/transform.rs` |
| OpenAPI docs (utoipa + Scalar UI) | ✅ Done | `src/openapi.rs` |
| Config extension (3 new fields) | ✅ Done | `src/config.rs` |
| Middleware stack integration | ✅ Done | `src/main.rs` |
| Auth public paths update | ✅ Done | `src/auth.rs` |

## New Dependencies Added

| Crate | Version | Purpose |
|:--|:--|:--|
| moka | 0.12 | High-perf async cache (future feature) |
| governor | 0.8 | GCRA-based rate limiting |
| utoipa | 5 | OpenAPI 3.1 spec generation |
| utoipa-scalar | 0.3 | Scalar API docs UI |

## Testing Summary

| Metric | Value |
|:--|:--|
| Tests passed | 22 |
| Tests failed | 0 |
| Compiler warnings | 0 |
| Clippy lints | 0 |

### Test Breakdown by Module
| Module | Tests | Coverage |
|:--|:--|:--|
| config | 1 | Default config values (incl. new fields) |
| health | 1 | Liveness probe |
| fhir | 3 | Router creation, path mapping, base path |
| cache | 4 | Creation, key generation, GET-only, insert/get |
| rate_limit | 5 | Creation, IP extraction (x-forwarded-for, x-real-ip, fallback), allow |
| transform | 4 | Path rewrite, no rewrite, query params, header injection |
| openapi | 4 | Spec generation, FHIR path, health paths, router |

## Architecture

```
Client → Eir Gateway (:9090) → OpenEMR PHP (:80)
         [CORS → Audit → RateLimit → Auth → Transform → Cache → FHIR|Proxy]

Endpoints:
  /healthz            — Liveness probe
  /readyz             — Readiness probe (upstream check)
  /fhir/r4/*          — FHIR R4 proxy → /apis/default/fhir/*
  /api-docs           — Scalar API documentation UI
  /api-docs/openapi.json — OpenAPI 3.1 JSON spec
  /*                  — General reverse proxy to OpenEMR
```

## New Config Variables

| Variable | Default | Description |
|:--|:--|:--|
| `RATE_LIMIT_RPS` | 100 | Max requests/sec per IP |
| `CACHE_TTL_SECS` | 60 | Cache TTL for GET responses |
| `TENANT_ID` | default | Tenant identifier header |

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-02)*

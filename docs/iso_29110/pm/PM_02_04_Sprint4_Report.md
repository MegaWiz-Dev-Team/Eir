# PM-02-04: Sprint 4 Report — Zitadel JWKS Auth
**Project Name:** Eir — API Gateway
**Sprint:** 4 (JWKS Auth)
**Date:** 2026-03-15
**Standard:** ISO/IEC 29110 — PM Process

---

## Sprint Goal
Replace static Bearer token auth with Zitadel JWKS-based JWT validation (RS256).

## Deliverables

| Item | Status | File |
|:--|:--|:--|
| `JwksCache` — fetch + cache JWKS keys | ✅ Done | `src/jwks.rs` |
| `ZitadelClaims` — JWT claims model | ✅ Done | `src/jwks.rs` |
| `validate()` — RS256 JWT validation | ✅ Done | `src/jwks.rs` |
| `auth.rs` rewrite — JWKS + static fallback | ✅ Done | `src/auth.rs` |
| `zitadel_issuer` + `jwt_audience` config | ✅ Done | `src/config.rs` |
| `jsonwebtoken = 9` dependency | ✅ Done | `Cargo.toml` |
| Version bump to v0.4.0 | ✅ Done | `Cargo.toml` |

## Testing Summary

| Metric | Value |
|:--|:--|
| New tests added | 8 |
| Total tests (cumulative) | 57 |
| Tests failed | 0 |
| Clippy warnings | 0 |
| Test time | 0.21s |

## Design Decisions
- **Fallback mode**: if `ZITADEL_ISSUER` is empty, falls back to static `AUTH_SECRET`
- **JWKS cache**: 1-hour refresh interval via tokio `RwLock`
- **Claims extraction**: `urn:zitadel:iam:org:id` → tenant_id

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-02)*

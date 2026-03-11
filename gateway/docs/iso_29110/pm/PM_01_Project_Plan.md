# PM-01: Project Plan — Eir Gateway
**Project Name:** Eir — Rust API Gateway for OpenEMR
**Document Version:** 1.0
**Date:** 2026-03-11
**Standard:** ISO/IEC 29110 — PM Process

---

## 1. Project Scope & Objectives

### เป้าหมาย
Rust-based API Gateway ทำหน้าที่ reverse proxy หน้า OpenEMR PHP backend เพิ่ม performance, security, audit logging, และ Asgard platform integration

### Tech Stack
| Layer | Technology |
|:--|:--|
| Runtime | Rust (edition 2021) |
| Web framework | Axum + Tokio |
| HTTP client | reqwest (async) |
| Logging | tracing + tracing-subscriber (JSON) |
| Middleware | tower-http (CORS, trace) |

---

## 2. Sprint Schedule

### Sprint 1: Foundation (Mar 11, 2026) — ✅ COMPLETED
| Deliverable | Status |
|:--|:--|
| Project scaffolding (Cargo.toml, .env) | ✅ Done |
| Config module (env-based) | ✅ Done |
| Health endpoints (/healthz, /readyz) | ✅ Done |
| Reverse proxy to OpenEMR | ✅ Done |
| Auth middleware (Bearer token) | ✅ Done |
| Audit logging middleware | ✅ Done |
| CORS middleware | ✅ Done |
| Tests | ✅ Done (2 passed, 0 warnings) |

### Sprint 2: FHIR Proxy + Enhancement (Planned)
| Deliverable | Status |
|:--|:--|
| FHIR resource proxy (/fhir/r4/*) | 📋 Planned |
| Response caching | 📋 Planned |
| Rate limiting | 📋 Planned |

### Sprint 3: Asgard Integration (Planned)
| Deliverable | Status |
|:--|:--|
| Bifrost agent tools | 📋 Planned |
| Mimir knowledge sync | 📋 Planned |

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-01)*

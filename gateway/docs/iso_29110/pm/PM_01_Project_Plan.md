# PM-01: Project Plan — Eir Gateway
**Project Name:** Eir — Rust API Gateway for OpenEMR
**Document Version:** 1.2
**Date:** 2026-03-12
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
| Caching | moka (async in-memory) |
| Rate Limiting | governor (GCRA) |
| API Docs | utoipa + Scalar UI |
| Timestamps | chrono (ISO 8601) |

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

### Sprint 2: FHIR Proxy + Enhancement (Mar 12, 2026) — ✅ COMPLETED
| Deliverable | Status |
|:--|:--|
| FHIR R4 proxy (/fhir/r4/*) | ✅ Done |
| Response caching (moka) | ✅ Done |
| Rate limiting (governor/GCRA) | ✅ Done |
| Request transformation | ✅ Done |
| OpenAPI docs (utoipa + Scalar UI) | ✅ Done |
| Tests | ✅ Done (22 passed, 0 warnings) |

### Sprint 3: Asgard Integration (Mar 12, 2026) — ✅ COMPLETED
| Deliverable | Status |
|:--|:--|
| Bifrost agent tools (FHIR query, patient search, clinical summary) | ✅ Done |
| Mimir knowledge sync (webhook + status) | ✅ Done |
| A2A protocol (Agent Card, task send/get) | ✅ Done |
| OpenAPI v0.3.0 (12 documented endpoints) | ✅ Done |
| Auth update (A2A public path) | ✅ Done |
| Tests | ✅ Done (47 passed, 0 warnings) |

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-01)*


# PM-01: Project Plan — Eir
**Project Name:** 🏥 Eir (Rust API Gateway + OpenEMR)
**Document Version:** 1.0
**Date:** 2026-03-16
**Standard:** ISO/IEC 29110 — PM Process
**Parent:** [Asgard PM-01](../../../Asgard/docs/iso_29110/pm/PM_01_Project_Plan.md)

---

## 1. Project Scope & Objectives

### เป้าหมาย
พัฒนา Rust API Gateway สำหรับ OpenEMR (FHIR R4) ที่สามารถ:
- Reverse proxy ไปยัง OpenEMR
- FHIR R4 aware routing + caching
- JWT authentication (Yggdrasil JWKS)
- Audit logging ตาม HIPAA/PDPA
- Chat widget สำหรับ AI assistant

### ขอบเขต

| Feature | Sprint | Priority |
|:--|:--|:--|
| Axum scaffold, reverse proxy, auth, audit, health | S1 | P0 |
| FHIR proxy, moka cache, rate limit, OpenAPI | S2 | P0 |
| Bifrost Agent Tools, Mimir Knowledge Sync, A2A | S3 | P0 |
| Yggdrasil JWKS Auth (RS256) | S4 | P0 |
| MCP Server (FHIR tools) + Embedded Chat UI | S5 | P1 |

---

## 2. Project Organization

| Role | Person/Team | Responsibility |
|:--|:--|:--|
| **Product Owner** | Paripol (MegaWiz) | Architecture, Rust backend |
| **Developer** | AI-assisted (Antigravity) | Implementation, testing |

---

## 3. Technical Architecture

| Layer | Technology |
|:--|:--|
| Language | Rust 2024 edition |
| Web Framework | Axum 0.8 |
| HTTP Client | reqwest 0.12 |
| Auth | jsonwebtoken 9 (RS256 JWKS) |
| Cache | moka (concurrent cache) |
| Rate Limit | governor |
| OpenEMR | Docker (contributedvolunteer/openemr) |
| Port | `:8300` |
| Container | `asgard_eir` |

### Source Structure

```
src/
├── main.rs        — Axum server + routes
├── config.rs      — Configuration management
├── auth.rs        — JWT auth (JWKS + static fallback)
├── jwks.rs        — JWKS cache + RS256 validation
├── proxy.rs       — Reverse proxy to OpenEMR
├── fhir.rs        — FHIR R4 aware routing
├── cache.rs       — Response caching (moka)
├── audit.rs       — Audit logging
├── health.rs      — Health + readiness
├── chat.rs        — Chat widget API
└── a2a.rs         — A2A protocol endpoints
```

---

- **Sprint 31: Mimir Hybrid Search & MCP Server Foundation** [Planned]
  - True Vector Integration, Parallel Tree Search, Neo4j Graph, Ensemble Retrieval, and Rust MCP Server.
- **Sprint 32: Asgard/Bifrost MCP Adapter & Dynamic Tenants** [Planned]
  - Auto-discover tools from MCP servers, Dynamic Context Isolation (X-Tenant-ID), Agent-to-Agent via JSON-RPC.
- **Sprint 33: Ecosystem Gateway Sidecars** [Planned]
  - Yggdrasil & Eir Universal Go Sidecars to expose auth and medical tools to Asgard.
- **Sprint 34: Platform Automation (Testing, Browsing & Security)** [Planned]
  - Deploy MCP across Fenrir, Forseti, Ratatoskr, Huginn, Muninn, and Heimdall.

## 4. Sprint Schedule

| Sprint | Duration | Deliverable | Tests | Status |
|:--|:--|:--|:--|:--|
| **S1** | 2 wk | Axum server, reverse proxy, auth, audit, health | 10 | ✅ Done |
| **S2** | 2 wk | FHIR R4 proxy, moka cache, rate limit, OpenAPI | 22 | ✅ Done |
| **S3** | 2 wk | Bifrost Agent Tools, Mimir Sync, A2A Protocol | 47 | ✅ Done (2026-03-12) |
| **S4** | 2 wk | Yggdrasil JWKS Auth (RS256) | 57 | ✅ Done (2026-03-15) |
| **S5** | 2 wk | MCP Server (FHIR tools), Embedded Chat UI | — | 📋 Planned |

---

## 5. Test Summary (Current)

| Metric | Value |
|:--|:--|
| Total tests | **57** |
| Tests passed | 57 |
| Tests failed | 0 |
| Clippy warnings | 0 |
| Test time | 0.21s |
| Framework | Rust `cargo test` |

---

## 6. Compliance

| Standard | Coverage |
|:--|:--|
| HIPAA Security Rule | Audit logging, JWT auth, encryption at rest |
| Thailand PDPA | PHI protection, consent tracking, data retention |
| FHIR R4 | Patient, Encounter, DocumentReference, MedicationRequest |

---

## 7. Risk Assessment

| Risk | Impact | Mitigation |
|:--|:--|:--|
| AGPL contamination from OpenEMR | High | Keep Eir isolated; no code merge with proprietary |
| JWKS key rotation | Medium | 1-hour cache refresh, fallback to static token |
| OpenEMR API breaking changes | Medium | Pin Docker image version, integration tests |
| PHI exposure in chat widget | High | Auth required, audit log all chat interactions |

---

*บันทึกโดย: AI Assistant (ISO/IEC 29110 PM-01)*
*Created: 2026-03-16 by Antigravity*

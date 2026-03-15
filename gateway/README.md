# 🏥 Eir Gateway — API Gateway for Asgard AI Platform

> Rust (Axum) API Gateway for OpenEMR with MCP Server, embedded Chat UI, and A2A agent protocol.

**Part of [Asgard AI Platform](https://github.com/megacare-dev/Asgard)**

## Features

| Feature | Description |
|:--|:--|
| 🔄 **Reverse Proxy** | Transparent proxy to OpenEMR (:80) |
| 💬 **Chat UI** | Embedded chat interface + 🐺 floating widget |
| 📡 **MCP Server** | FHIR tools for Bifrost agent (patient search, clinical summary) |
| 🤖 **A2A Protocol** | Agent-to-Agent task delegation |
| ⏱️ **Rate Limiting** | Governor-based RPS limiter |
| 📦 **Caching** | Moka in-memory cache for FHIR responses |
| 🔐 **Auth** | Bearer token + tenant header injection |
| 📋 **Audit Log** | Request/response logging |

## Architecture

```
                    ┌── Eir Gateway (:8300) ──┐
  User ──→         │                          │
  Bifrost (MCP) ──→│  /chat          Chat UI  │
  Bifrost (A2A) ──→│  /v1/chat       → Bifrost│
                    │  /v1/patients   MCP tool │
                    │  /v1/fhir       MCP tool │
                    │  /.well-known   A2A card │
                    │  /*             → proxy  │──→ OpenEMR (:80)
                    └─────────────────────────┘
```

## Integration Protocols

### MCP — Tool Calls (Bifrost → Eir)
```
Bifrost calls Eir MCP tools:
  - patient_search("สมชาย")      → FHIR Patient resources
  - fhir_query("Condition?...")  → FHIR query results
  - clinical_summary(patient_id) → Combined patient data
```

### A2A — Task Delegation (Bifrost → Eir Agent)
```
Bifrost sends complex tasks:
  POST /a2a/tasks/send
  { skill: "patient-registration",
    message: "ลงทะเบียนคนไข้ สมชาย อายุ 45" }
  → Eir Agent plans + executes multi-step workflow
```

## Endpoints

| Endpoint | Method | Description |
|:--|:--|:--|
| `/healthz` | GET | Health check |
| `/readyz` | GET | Readiness (includes OpenEMR) |
| `/chat` | GET | Chat UI page |
| `/v1/chat` | POST | Chat proxy → Bifrost |
| `/v1/chat/status` | GET | Bifrost connectivity |
| `/v1/patients/search` | GET | Patient search (MCP) |
| `/v1/fhir/query` | POST | FHIR natural language query |
| `/v1/clinical/summary` | POST | Clinical summary |
| `/.well-known/agent.json` | GET | A2A agent card |
| `/a2a/tasks/send` | POST | A2A task submission |
| `/docs` | GET | OpenAPI UI (Scalar) |
| `/*` | ANY | Reverse proxy → OpenEMR |

## Quick Start

```bash
# Via Docker Compose (recommended)
cd Asgard
docker compose --profile full up -d

# Access
open http://localhost:8300/chat    # Chat UI
open http://localhost:8300/        # OpenEMR (via gateway)
open http://localhost:8300/docs    # API docs
```

## Environment Variables

| Variable | Default | Description |
|:--|:--|:--|
| `OPENEMR_URL` | `http://eir:80` | Upstream OpenEMR URL |
| `EIR_GATEWAY_PORT` | `8300` | Gateway listen port |
| `BIFROST_URL` | `http://bifrost:8100` | Bifrost agent runtime |
| `AUTH_ENABLED` | `false` | Enable bearer auth |
| `RATE_LIMIT_RPS` | `100` | Requests per second |
| `CACHE_TTL_SECS` | `60` | Cache TTL |

## Sprint History

| Sprint | Deliverable | Tests |
|:--|:--|:--|
| Sprint 1 | Axum server, reverse proxy, auth, audit, health | 10 |
| Sprint 2 | FHIR R4 proxy, moka cache, governor rate limit, OpenAPI | 22 |
| Sprint 3 | Bifrost Agent Tools, Mimir Knowledge Sync, A2A Protocol | 47 |
| Sprint 3.5 | Embedded Chat UI + 🐺 Floating Widget (mint green theme) | 47 |

## License

AGPL-3.0 — Part of Asgard AI Platform

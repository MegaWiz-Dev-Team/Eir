# Eir Sprint Tasks & Prompts
## รวม Task ทั้งหมดของทุก Sprint พร้อม Prompt สำหรับเริ่มงาน

---

## Sprint 1: Foundation & Proxy Core
**Status:** ✅ Done | **Period:** 2026-03-11 | **Tests:** 10

### Tasks
- [x] สร้าง Axum server entry point (`src/main.rs`)
- [x] สร้าง Environment config (`src/config.rs`)
- [x] สร้าง Health endpoints GET /healthz, /readyz (`src/health.rs`)
- [x] สร้าง Reverse proxy ไปยัง OpenEMR (`src/proxy.rs`)
- [x] สร้าง Auth middleware — Bearer token validation (`src/auth.rs`)
- [x] สร้าง Audit logging middleware (`src/audit.rs`)
- [x] เขียน Unit Tests (2 tests)

### Prompt
```
สร้าง Rust API Gateway สำหรับ OpenEMR ด้วย Axum 0.8 ที่ทำหน้าที่เป็น Reverse Proxy

ต้องมี:
1. Axum server ที่ listen port 9090
2. Reverse proxy ส่งต่อ request ไป OpenEMR ที่ port 80
3. Auth middleware ตรวจ Bearer token จาก ENV AUTH_SECRET
4. Audit logging บันทึกทุก request (method, path, status, duration)
5. Health endpoints: GET /healthz (liveness), GET /readyz (readiness ตรวจ upstream)
6. CORS middleware
7. Config จาก environment variables

Architecture: Client → Eir Gateway (:9090) → [CORS → Audit → Auth → Proxy] → OpenEMR (:80)

เขียน Unit Tests ให้ครบทุก module, ใช้ cargo test ผ่านหมด, clippy ไม่มี warning
```

---

## Sprint 2: FHIR Proxy + Enhancement
**Status:** ✅ Done | **Period:** 2026-03-12 | **Tests:** 22

### Tasks
- [x] สร้าง FHIR R4 proxy route `/fhir/r4/*` → `/apis/default/fhir/*` (`src/fhir.rs`)
- [x] สร้าง Response caching ด้วย moka (`src/cache.rs`)
- [x] สร้าง Rate limiting ด้วย governor/GCRA (`src/rate_limit.rs`)
- [x] สร้าง Request transformation — path rewrite + header injection (`src/transform.rs`)
- [x] สร้าง OpenAPI docs ด้วย utoipa + Scalar UI (`src/openapi.rs`)
- [x] อัปเดต Config เพิ่ม RATE_LIMIT_RPS, CACHE_TTL_SECS, TENANT_ID (`src/config.rs`)
- [x] อัปเดต Auth public paths (`src/auth.rs`)
- [x] เขียน Unit Tests (22 tests รวม)

### Prompt
```
ต่อยอด Eir Gateway จาก Sprint 1 เพิ่มฟีเจอร์ดังนี้:

1. FHIR R4 Proxy: เพิ่ม route /fhir/r4/* ที่ rewrite path ไป /apis/default/fhir/* ส่งต่อไป OpenEMR
2. Response Caching: ใช้ moka 0.12 cache GET response, TTL ตั้งค่าได้จาก ENV CACHE_TTL_SECS (default 60)
3. Rate Limiting: ใช้ governor 0.8 แบบ GCRA, limit ตั้งค่าได้จาก ENV RATE_LIMIT_RPS (default 100), ดึง IP จาก X-Forwarded-For → X-Real-IP → socket
4. Request Transformation: rewrite path + inject X-Tenant-ID header จาก ENV TENANT_ID
5. OpenAPI Documentation: ใช้ utoipa 5 + utoipa-scalar 0.3, serve ที่ /api-docs (Scalar UI) + /api-docs/openapi.json

Middleware Stack: [CORS → Audit → RateLimit → Auth → Transform → Cache → FHIR|Proxy]

เขียน Unit Tests ครอบคลุมทุก module ใหม่ (cache 4, rate_limit 5, transform 4, openapi 4, fhir 3)
cargo test ผ่านหมด, clippy ไม่มี warning
```

---

## Sprint 3: Asgard Integration
**Status:** ✅ Done | **Period:** 2026-03-12 | **Tests:** 47

### Tasks
- [x] สร้าง Bifrost agent tools — 3 endpoints: FHIR query, patient search, clinical summary (`src/agent_tools.rs`)
- [x] สร้าง Mimir knowledge sync — webhook + status (`src/knowledge.rs`)
- [x] สร้าง A2A protocol — Agent Card + task send/get/list (`src/a2a.rs`)
- [x] อัปเดต OpenAPI spec เพิ่ม paths ใหม่ทั้งหมด (`src/openapi.rs`)
- [x] อัปเดต Auth public paths (`src/auth.rs`)
- [x] อัปเดต Router integration (`src/main.rs`)
- [x] Version bump 0.2.0 → 0.3.0 (`Cargo.toml`)
- [x] เขียน Unit Tests (47 tests รวม: agent_tools 7, knowledge 6, a2a 9)

### Prompt
```
ต่อยอด Eir Gateway จาก Sprint 2 เพิ่ม Asgard ecosystem integration:

1. Bifrost Agent Tools (3 endpoints):
   - POST /v1/fhir/query — FHIR query tool สำหรับ Bifrost agent
   - POST /v1/patients/search — Patient search tool
   - POST /v1/clinical/summary — Clinical summary tool
   สร้าง DTOs ด้วย serde, ส่ง proxy query ไป OpenEMR FHIR API

2. Mimir Knowledge Sync:
   - POST /v1/webhooks/mimir — รับ webhook event จาก Mimir (knowledge update)
   - GET /v1/knowledge/status — ดูสถานะ sync
   ใช้ in-memory store, บันทึก source + timestamp, ป้องกัน duplicates

3. A2A Protocol (Google Agent-to-Agent):
   - GET /.well-known/agent.json — Agent Card (capabilities, skills)
   - POST /a2a/tasks/send — สร้าง task ใหม่
   - GET /a2a/tasks/{id} — ดู task status
   - GET /a2a/tasks — list ทุก task
   ใช้ in-memory task store, state: submitted → working → completed

Dependencies: chrono 0.4 สำหรับ ISO 8601 timestamps
Version bump → 0.3.0
เขียน Unit Tests ครอบคลุมทุก module ใหม่ (agent_tools 7, knowledge 6, a2a 9)
```

---

## Sprint 4: Yggdrasil JWKS Auth
**Status:** ✅ Done | **Period:** 2026-03-15 | **Tests:** 57

### Tasks
- [x] สร้าง JwksCache — fetch + cache JWKS keys จาก Yggdrasil (`src/jwks.rs`)
- [x] สร้าง YggdrasilClaims — JWT claims model (`src/jwks.rs`)
- [x] สร้าง validate() — RS256 JWT validation (`src/jwks.rs`)
- [x] Rewrite auth.rs — JWKS validation + static token fallback (`src/auth.rs`)
- [x] เพิ่ม Config: `yggdrasil_issuer`, `jwt_audience` (`src/config.rs`)
- [x] เพิ่ม dependency: jsonwebtoken 9 (`Cargo.toml`)
- [x] Version bump → 0.4.0 (`Cargo.toml`)
- [x] เขียน Unit Tests (57 tests รวม, เพิ่ม 8 tests ใหม่)

### Prompt
```
ต่อยอด Eir Gateway จาก Sprint 3 แทนที่ static Bearer token ด้วย Yggdrasil JWKS:

1. JWKS Cache (src/jwks.rs):
   - Fetch JWKS keys จาก Yggdrasil issuer URL: {YGGDRASIL_ISSUER}/.well-known/jwks.json
   - Cache keys ด้วย tokio RwLock, refresh ทุก 1 ชม.
   - สร้าง YggdrasilClaims struct (sub, aud, iss, exp, urn:zitadel:iam:org:id → tenant_id)

2. Auth Rewrite (src/auth.rs):
   - ถ้า YGGDRASIL_ISSUER ตั้งค่าไว้ → validate JWT ด้วย RS256 JWKS
   - ถ้า YGGDRASIL_ISSUER ว่าง → fallback ไป static AUTH_SECRET เหมือนเดิม
   - Extract tenant_id จาก claims แล้วใส่ใน request extensions

3. Config (src/config.rs):
   - เพิ่ม YGGDRASIL_ISSUER (default: empty)
   - เพิ่ม JWT_AUDIENCE (default: empty)

Dependencies: jsonwebtoken = "9"
Version bump → 0.4.0

เขียน Unit Tests 8 tests ใหม่สำหรับ JWKS + fallback, cargo test ผ่านหมด
```

---

## Sprint 5: 🔴 PDPA Consent & Data Migration Readiness
**Status:** ✅ Done | **Period:** 2026-03-22 → 23 | **Tests:** 40 (Forseti run_id=30)

### Tasks
**Day 22 (Documentation):**
- [x] วิเคราะห์ Data Schema Mapping (Mega Care → Eir)
- [x] จัดทำ Migration Safety Review & Archive Plan
- [x] จัดทำ PDPA Consent Remediation Plan
- [x] ตรวจ Codebase พบ Gap: Agent ไม่เช็ค consent

**Day 22-23 (Consent Gate + Sandbox):**
- [x] เพิ่ม Consent Gate ใน Extraction Agent (`job_runner.py`)
- [x] เพิ่ม Consent Gate ใน `processReportUpload` (`report_processor/main.py`)
- [x] เพิ่ม Consent Gate ใน Daily Extraction Trigger (`daily_extraction_trigger/main.py`)
- [x] สร้าง Script ตั้ง `consent.status = "pending"` (`set_pending_consent.py`)
- [x] เขียน Unit Tests สำหรับ Consent Gate (37 tests report processor + 3 tests daily trigger)
- [x] สร้าง `sites/sandbox/` + DB `openemr_sandbox`
- [x] Seed Mock Data (5 คนไข้สมมติ)

**Day 23 (Eir Schema + API):**
- [x] สร้าง LBF Form "CPAP_Prescription" (`sql/migrations/002_lbf_cpap_prescription.sql`)
- [x] สร้าง LBF Form "Sleep_Report_Data" (`sql/migrations/003_lbf_sleep_report.sql`)
- [x] สร้าง `migration_log` table (`sql/migrations/001_migration_log.sql`)
- [x] พัฒนา Custom API `POST /cpap-sync` (`CpapSyncRestController.php`)
- [x] เขียน PHPUnit Tests (11 test cases)
- [x] พัฒนา e-Consent Form (Line OA Flex Message + Cloud Function webhook)

**Day 23 (CI/CD + Testing + ISO):**
- [x] สร้าง CI/CD Pipeline (`cloudbuild.yaml` for Mega Care)
- [x] สร้าง Deploy Script (`scripts/deploy.sh` for Eir)
- [x] Refactor `report_processor/main.py` ให้ testable (lazy init `get_db()`, `get_storage_client()`)
- [x] รัน Tests: 40/40 passed (100%) — Forseti run_id=30
- [x] อัพเดต ISO Docs: `SI_01_Implementation_Report.md` v0.2.0

**Remaining (Manual/Ops — ไม่ต้อง code):**
- [ ] Deploy Consent Gate ขึ้น Production (GCP: `mega-care`)
- [ ] รัน `set_pending_consent.py` กับ Firestore production
- [ ] ส่ง Consent Request Batch 1 (Line OA)
- [ ] Monitor Cloud Logging

### Environment
| สภาพแวดล้อม | Mega Care (GCP) | Eir Site | Firestore |
|-----------|-----------------|----------|-----------|
| Production | `mega-care` | `sites/default/` → DB: `openemr` | `mega-care-db` |
| Development | `mega-care-dev` | `sites/sandbox/` → DB: `openemr_sandbox` | `mega-care-dev-db` |

### Prompt
```
Sprint 5 ของ Eir: PDPA Consent Remediation + Data Migration Readiness
ระยะเวลา 4 วัน (22-26 มี.ค.) — URGENT เรื่องกฎหมาย PDPA

สถานะปัจจุบัน:
- mega-care-admin-portal มี API update consent แล้ว (PUT /{patient_id}/consent)
- มี Right to be Forgotten endpoint แล้ว
- Dashboard แสดง consent_status แล้ว
- 🔴 แต่ Extraction Agent ดูดข้อมูล AirView โดยไม่เช็ค consent เลย
- 🔴 Cloud Functions processReportUpload ไม่เช็ค consent
- 🔴 คนไข้ทั้งหมดยังไม่ได้ให้ consent

สิ่งที่ต้องทำ:

1. Consent Gate (3 จุด):
   - agent/: ก่อนดึงข้อมูลจาก AirView ให้เช็ค patient.consent.status == "active"
   - cloud-functions/processReportUpload: เช็ค consent ก่อนประมวลผล Report
   - daily-patient-extraction-trigger/: เพิ่ม .where('consent.status', '==', 'active') ใน query
   ถ้า consent ≠ active → SKIP + log

2. Sandbox Setup (Eir):
   - สร้าง sites/sandbox/ (copy จาก sites/default/)
   - สร้าง DB openemr_sandbox, แก้ sqlconf.php ให้ชี้ DB ใหม่
   - Seed Mock Data 5 คนไข้: Green/Yellow/Red/Declined/Pending
   - ใช้ GCP Project "mega-care-dev" เชื่อมกับ Sandbox

3. Eir Schema:
   - สร้าง LBF Form "CPAP_Prescription" (fields: cpap_model, cpap_sn, therapy_mode, pressure_min/max, epr_level, mask_type)
   - สร้าง LBF Form "Sleep_Report_Data" (fields: date, usage_hours, ahi, leak_95th, triage_status)
   - สร้าง migration_log table (patient_id, data_type, source_doc_id, status, error_message)
   - สร้าง API POST /cpap-sync รับ JSON payload จาก Mega Care

4. e-Consent:
   - สร้าง Line OA Flex Message ให้คนไข้กดยินยอม/ไม่ยินยอม
   - บันทึก consent ลง Firestore: status, version, channel, consented_at

5. CI/CD:
   - Cloud Build: Lint → Test (mega-care-dev) → PR Review → Deploy (mega-care)
   - ทุก feature branch ต้องทดสอบบน Sandbox ก่อน merge

6. ทดสอบ T1-T8 บน Sandbox ก่อน Deploy Production

เอกสารอ้างอิง:
- Eir/docs/Data_Mapping_MegaCare_to_Eir.md
- Eir/docs/PDPA_Consent_Remediation_Plan.md
- Eir/docs/Sandbox_and_Production_Environment_Plan.md
- Eir/docs/Migration_Safety_Review_and_Archive_Plan.md
```

---

## Sprint 6: MCP Server + Embedded Chat UI
**Status:** 📋 Planned | **Period:** TBD | **Tests:** TBD

### Tasks
- [ ] สร้าง MCP Server สำหรับ FHIR tools (expose Patient, Encounter, MedicationRequest)
- [ ] สร้าง MCP Tool: `search_patients` — ค้นหาคนไข้ผ่าน FHIR
- [ ] สร้าง MCP Tool: `get_patient_summary` — ดึงสรุปข้อมูลคนไข้
- [ ] สร้าง MCP Tool: `create_encounter` — สร้าง Encounter ใหม่
- [ ] สร้าง MCP Tool: `get_sleep_reports` — ดึง Sleep Report data (จาก LBF)
- [ ] สร้าง Embedded Chat Widget UI (HTML + JS)
- [ ] เชื่อม Chat Widget → Bifrost → MCP Server → Eir FHIR API
- [ ] สร้าง RBAC สำหรับ Chat (หมอ vs พยาบาล vs Admin)
- [ ] อัปเดต OpenAPI spec
- [ ] เขียน Unit Tests + Integration Tests
- [ ] Version bump → 0.5.0

### Prompt
```
Sprint 6 ของ Eir: MCP Server (FHIR Tools) + Embedded Chat UI

สถานะปัจจุบัน:
- Eir Gateway v0.4.0 มี FHIR proxy, Auth (JWKS), A2A protocol, Agent Tools
- Sprint 5 เสร็จ: มี LBF Forms (CPAP, Sleep Report), Consent Gate, Sandbox
- Data Migration จาก Mega Care กำลังดำเนินการ (Dual-Write)

สิ่งที่ต้องทำ:

1. MCP Server (src/mcp.rs):
   - Implement MCP protocol (Model Context Protocol) ตามมาตรฐาน
   - ให้ Eir เป็น MCP Server ที่ Asgard/Bifrost เรียกใช้ tools ได้
   - Tools:
     a. search_patients(query: str) → FHIR Patient search results
     b. get_patient_summary(patient_id: str) → สรุปข้อมูลคนไข้ (demographics + CPAP + sleep data)
     c. create_encounter(patient_id: str, type: str) → สร้าง Encounter ใหม่
     d. get_sleep_reports(patient_id: str, days: int) → ดึง Sleep Report data จาก LBF form

2. Embedded Chat Widget:
   - สร้าง HTML + JS chat widget inject เข้าหน้า OpenEMR
   - Chat → Bifrost Agent → MCP Tools → Eir FHIR API
   - UI: ปุ่มลอย มุมล่างขวา, expand เป็น chat window
   - เพียงแค่ถาม "สรุปข้อมูลคนไข้ SANDBOX-PT-001" ได้

3. RBAC สำหรับ Chat:
   - หมอ: เข้าถึงทุก tool
   - พยาบาล: เข้าถึง search + summary (ไม่สร้าง encounter)
   - Admin: ไม่เข้าถึง clinical data

4. Audit Trail: ทุก MCP call → log ไว้ใน audit table

Version bump → 0.5.0
เขียน Unit Tests + Integration Tests ครบทุก tool
```

---

*บันทึกโดย: AI Assistant | อัปเดตล่าสุด: 2026-03-22*

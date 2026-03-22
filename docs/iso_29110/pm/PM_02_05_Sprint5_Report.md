# PM-02-05: Sprint 5 Report — PDPA Consent & Data Migration Readiness
**Project Name:** Eir + Mega Care Admin Portal (Cross-Project Sprint)
**Sprint:** 5 (PDPA Consent Remediation & Clinical Data Migration Prep)
**Period:** 2026-03-22 → 2026-03-26 (4 วัน)
**Standard:** ISO/IEC 29110 — PM Process
**Priority:** 🔴 URGENT — กฎหมาย PDPA

---

## Sprint Goal
อุดช่องโหว่ด้าน PDPA Consent ที่ยังไม่ได้ขอความยินยอมจากคนไข้ และเตรียมโครงสร้าง Eir ให้พร้อมรับข้อมูลจาก Mega Care เพื่อเริ่ม Data Migration พร้อมวาง CI/CD Pipeline สำหรับ Sandbox-to-Production workflow

### Environment Mapping
| สภาพแวดล้อม | Mega Care (GCP Project) | Eir (OpenEMR Site) | Firestore DB | ใช้สำหรับ |
|-----------|-------------------------|--------------------|--------------|---------|
| **Production** | `mega-care` | `sites/default/` → DB: `openemr` | `mega-care-db` | ข้อมูลจริง, Deploy หลังผ่าน Sandbox Test |
| **Development** | `mega-care-dev` | `sites/sandbox/` → DB: `openemr_sandbox` | `mega-care-dev-db` | Dry Run, Mock Data, ทดสอบ T1-T8 |

---

## CI/CD Pipeline Strategy

### Git Branching Model
```
main (Production-ready, protected branch)
 │
 ├── feature/consent-gate          ← โค้ด Consent Gate 3 จุด
 ├── feature/cpap-sync-api         ← Custom API ฝั่ง Eir
 ├── feature/e-consent-form        ← Line OA Consent Form
 │
 ├── migrations/
 │     ├── 001_migration_log.sql      ← Schema changes
 │     └── 002_lbf_cpap.sql           ← LBF Export from Sandbox
 │
 └── seed/
       └── sandbox_mock_data.sql      ← Mock Data (เฉพาะ Sandbox)
```

### Pipeline Flow (Sandbox → Production)
```
┌─────────────────────────────────────────────────────────┐
│  Stage 1: BUILD & LINT                                │
│  Trigger: Push to feature/* branch                    │
│  ───────────────────────────────────────────────────── │
│  • PHP lint (phpcs / phpstan)                          │
│  • Python lint (ruff) ← Mega Care backend              │
│  • JS lint (eslint) ← Frontend                         │
└─────────────────────────────────────────────────────────┘
                         │ ✅ Pass
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Stage 2: TEST on SANDBOX                             │
│  ───────────────────────────────────────────────────── │
│  • รัน Unit Tests (Consent Gate, API)                   │
│  • รัน Integration Tests T1-T8 บน ?site=sandbox        │
│  • Reconciliation Check (COUNT match)                  │
│  • Consent Gate Validation (คนไข้ declined ถูก skip)   │
└─────────────────────────────────────────────────────────┘
                         │ ✅ Pass
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Stage 3: APPROVE (Manual Gate)                        │
│  ───────────────────────────────────────────────────── │
│  • Create Pull Request → main                         │
│  • PO / Tech Lead review code                          │
│  • ตรวจ Test Results ผ่านหมด + Lint ผ่าน             │
│  • Approve & Merge                                     │
└─────────────────────────────────────────────────────────┘
                         │ ✅ Merged
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Stage 4: DEPLOY to PRODUCTION                         │
│  Trigger: Merge to main                               │
│  ───────────────────────────────────────────────────── │
│  Mega Care:                                           │
│  • Cloud Build → Deploy Cloud Functions                │
│  • Cloud Build → Deploy Backend (Cloud Run)            │
│  Eir:                                                 │
│  • รัน SQL Migrations บน Production DB (openemr)        │
│  • Restart PHP-FPM / Apache                            │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Stage 5: POST-DEPLOY MONITORING                       │
│  ───────────────────────────────────────────────────── │
│  • ตรวจ Cloud Logging 1 ชม. (ดู Consent Gate SKIP)     │
│  • ตรวจ Error Rate ไม่เพิ่มขึ้นหลัง Deploy                │
│  • Rollback หากพบปัญหา (ใช้ Cloud Run Revisions)     │
└─────────────────────────────────────────────────────────┘
```

### Cloud Build Config (ตัวอย่าง สำหรับ Mega Care Backend)
```yaml
# cloudbuild.yaml (mega-care-admin-portal)
# Deploy target: GCP Project "mega-care" (Production)
# Dev/Test target: GCP Project "mega-care-dev" (เชื่อมกับ Eir Sandbox)
steps:
  # Step 1: Lint
  - name: 'python:3.11'
    entrypoint: 'bash'
    args: ['-c', 'pip install ruff && ruff check backend/']

  # Step 2: Unit Tests (รันบน mega-care-dev)
  - name: 'python:3.11'
    entrypoint: 'bash'
    args: ['-c', 'pip install -r backend/requirements.txt && pytest backend/tests/ -v']
    env:
      - 'GCP_PROJECT=mega-care-dev'
      - 'EIR_SITE=sandbox'

  # Step 3: Deploy Backend to Cloud Run (Production: mega-care)
  - name: 'gcr.io/google.com/cloudsdktool/cloud-sdk'
    args: ['gcloud', 'run', 'deploy', 'mega-care-backend',
           '--project', 'mega-care',
           '--source', 'backend/',
           '--region', 'asia-southeast1',
           '--set-env-vars', 'GCP_PROJECT=mega-care,EIR_SITE=default']

  # Step 4: Deploy Cloud Functions (Production: mega-care)
  - name: 'gcr.io/google.com/cloudsdktool/cloud-sdk'
    args: ['gcloud', 'functions', 'deploy', 'processReportUpload',
           '--project', 'mega-care',
           '--source', 'cloud-functions/processReportUpload/',
           '--runtime', 'python311',
           '--trigger-event', 'google.storage.object.finalize']
```

## Day-by-Day Plan (22-26 มี.ค.)

### 📅 วันที่ 22 มี.ค. (วันนี้) — Documentation & Planning
| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | ✅ วิเคราะห์ Data Schema (Mega Care → Eir Mapping) | AI | ✅ Done |
| 2 | ✅ จัดทำ Migration Safety Review & Archive Plan | AI | ✅ Done |
| 3 | ✅ จัดทำ PDPA Consent Remediation Plan | AI | ✅ Done |
| 4 | ✅ ตรวจ Codebase พบ Gap: Agent ไม่เช็ค consent | AI | ✅ Done |

### 📅 วันที่ 23 มี.ค. — Consent Gate Implementation + Sandbox Setup
| # | Task | Owner | Status | File |
|---|------|-------|--------|------|
| 5 | เพิ่ม Consent Gate ใน Extraction Agent | Dev Backend | ✅ Done | `packages/shared/shared/services/job_runner.py` |
| 6 | เพิ่ม Consent Gate ใน `processReportUpload` | Dev Backend | ✅ Done | `cloud_functions/report_processor/main.py` |
| 7 | เพิ่ม Consent Gate ใน Daily Extraction Trigger | Dev Backend | ✅ Done | `cloud_functions/daily_extraction_trigger/main.py` |
| 8 | รัน Script ตั้ง `consent.status = "pending"` สำหรับคนไข้เก่าทุกคน | Dev Backend | ✅ Done | `scripts/set_pending_consent.py` |
| 9 | เขียน Unit Tests สำหรับ Consent Gate ทั้ง 3 จุด | Dev Backend | ✅ Done | `tests/` |
| 10 | สร้าง `sites/sandbox/` + DB `openemr_sandbox` | Dev Eir | ✅ Done | `sites/sandbox/sqlconf.php` |
| 11 | Seed Mock Data (5 คนไข้สมมติ) ลง Sandbox | Dev Eir | ✅ Done | `sql/seed/sandbox_mock_data.sql` |

### 📅 วันที่ 24 มี.ค. — Eir Schema & API Preparation
| # | Task | Owner | Status | File |
|---|------|-------|--------|------|
| 10 | สร้าง LBF Form "CPAP_Prescription" ใน OpenEMR | Dev Eir | ✅ Done | `sql/migrations/002_lbf_cpap_prescription.sql` |
| 11 | สร้าง LBF Form "Sleep_Report_Data" ใน OpenEMR | Dev Eir | ✅ Done | `sql/migrations/003_lbf_sleep_report.sql` |
| 12 | สร้าง `migration_log` table ใน MySQL | Dev Eir | ✅ Done | `sql/migrations/001_migration_log.sql` |
| 13 | พัฒนา Custom API Endpoint `POST /cpap-sync` | Dev Eir | ✅ Done | `src/RestControllers/CpapSyncRestController.php` |
| 14 | พัฒนา e-Consent Form (Line OA Flex Message) | Dev Frontend | ✅ Done | `customer-service/consent_flex_message.json` |

### 📅 วันที่ 25 มี.ค. — Integration Testing on Sandbox
| # | Task | Owner | Status | File |
|---|------|-------|--------|------|
| 17 | Dry Run: ทดสอบ Consent Gate บน Sandbox | QA / Dev | ⬜ | Sandbox env |
| 18 | Dry Run: ทดสอบ Dual-Write (Mega Care → Eir Sandbox) | QA / Dev | ⬜ | Sandbox env |
| 19 | Dry Run: ทดสอบ e-Consent Flow | QA / Dev | ⬜ | Sandbox env |
| 20 | รัน Test Scenarios T1-T8 ทั้งหมดบน Sandbox | QA / Dev | ⬜ | Sandbox env |
| 21 | เตรียม Consent Request Message Template | Ops / Legal | ⬜ | ขอ Review จากทนาย |

### 📅 วันที่ 26 มี.ค. — Deploy & Go-Live
| # | Task | Owner | Status | File |
|---|------|-------|--------|------|
| 19 | Deploy Consent Gate (3 จุด) ขึ้น Production | DevOps | ⬜ | Cloud Build |
| 20 | Deploy e-Consent Form ขึ้น Production | DevOps | ⬜ | Cloud Build |
| 21 | ส่ง Consent Request Batch 1 (คนไข้ที่มี Line ID) | Ops | ⬜ | Line OA |
| 22 | Monitor: ตรวจ Cloud Logging ว่า Gate ทำงานถูกต้อง | DevOps | ⬜ | GCP Console |
| 23 | จัดทำ Sprint 5 Report (ISO 29110 PM-02) | AI / PO | ⬜ | This file |

---

## Deliverables Summary

| # | Deliverable | Priority | Target |
|---|-------------|----------|--------|
| D1 | Consent Gate ใน Extraction Pipeline (3 จุด) | 🔴 P0 | 23 มี.ค. |
| D2 | LBF Forms (CPAP + Sleep Report) ใน Eir | 🟡 P1 | 24 มี.ค. |
| D3 | Custom Inbound API (`/cpap-sync`) ใน Eir | 🟡 P1 | 24 มี.ค. |
| D4 | e-Consent Form (Line OA) | 🔴 P0 | 24 มี.ค. |
| D5 | Migration Log Table ใน Eir | 🟡 P1 | 24 มี.ค. |
| D6 | CI/CD Pipeline (Cloud Build + Sandbox Gate) | 🔴 P0 | 25 มี.ค. |
| D7 | Consent Campaign Batch 1 Sent | 🔴 P0 | 26 มี.ค. |

---

## Risk Assessment (Sprint-specific)

| Risk | Impact | Mitigation |
|:--|:--|:--|
| ทนายยังไม่ Review Consent Form ทัน | 🔴 High | ใช้ Template มาตรฐาน PDPA + ส่งให้ Review ขนานไป |
| คนไข้เก่าไม่ตอบ consent | 🟡 Medium | ตั้ง Reminder อัตโนมัติ 2 ครั้ง (7 วัน, 14 วัน) |
| Consent Gate ทำให้ Daily Report หยุดทำงาน | 🟡 Medium | Sandbox Test T1-T8 + Cloud Logging monitor |
| LBF Form ไม่พร้อมภายใน 1 วัน | 🟢 Low | สามารถใช้ Generic Form ชั่วคราวได้ |
| CI/CD Deploy พังบน Production | 🔴 High | Rollback ผ่าน Cloud Run Revisions (สับกลับ version เดิมได้ใน 30 วินาที) |

---

## เอกสารอ้างอิงที่จัดทำแล้ว (ใน Sprint นี้)

| # | เอกสาร | Path |
|---|--------|------|
| 1 | Operation Flow (Apnea Care) | `Eir/docs/Apnea_Care_Operation_Flow.md` |
| 2 | Gap Analysis (Telemedicine) | `Eir/docs/Apnea_Care_Gap_Analysis.md` |
| 3 | Data Migration Plan | `Eir/docs/Apnea_Data_Migration_Plan.md` |
| 4 | Data Schema Mapping | `Eir/docs/Data_Mapping_MegaCare_to_Eir.md` |
| 5 | Migration Safety Review & Archive | `Eir/docs/Migration_Safety_Review_and_Archive_Plan.md` |
| 6 | PDPA Consent Remediation Plan | `Eir/docs/PDPA_Consent_Remediation_Plan.md` |

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-02)*

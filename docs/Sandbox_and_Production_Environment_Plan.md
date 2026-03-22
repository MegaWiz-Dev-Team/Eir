# Eir Sandbox & Production Environment Plan
## แผนการแยกสภาพแวดล้อม Sandbox (Mock Data) และ Production (Data จริง)

---

## 1. สถาปัตยกรรมการแยกสภาพแวดล้อม (Architecture)

```
┌────────────────────────────────────────────────────────┐
│                    Eir (OpenEMR)                       │
│                   Same Codebase (PHP)                  │
│                                                        │
│  ┌──────────────────┐    ┌───────────────────────────┐ │
│  │  sites/default/  │    │     sites/sandbox/        │ │
│  │  (Production)    │    │     (Sandbox / Dev)       │ │
│  │                  │    │                           │ │
│  │  sqlconf.php     │    │  sqlconf.php              │ │
│  │  → DB: openemr   │    │  → DB: openemr_sandbox    │ │
│  │                  │    │                           │ │
│  │  ข้อมูลจริง       │    │  Mock Data (Seeded)       │ │
│  │  คนไข้จริง        │    │  คนไข้สมมติ               │ │
│  │  ห้ามทดสอบ        │    │  ทดสอบได้อิสระ             │ │
│  └──────────────────┘    └───────────────────────────┘ │
│       │                            │                     │
│       ▼                            ▼                     │
│  GCP: mega-care              GCP: mega-care-dev         │
│  (Production)                (Development)              │
│  Firestore: mega-care-db     Firestore: mega-care-dev-db│
└────────────────────────────────────────────────────────┘
```

---

## 2. ขั้นตอนการตั้งค่า Sandbox

### Step 1: สร้างโฟลเดอร์ Sandbox Site
```bash
cd /Users/mimir/Developer/Eir
cp -r sites/default sites/sandbox
```

### Step 2: สร้าง Database สำหรับ Sandbox
```sql
-- รันบน MySQL Server
CREATE DATABASE openemr_sandbox
  CHARACTER SET utf8mb4
  COLLATE utf8mb4_general_ci;

-- ให้สิทธิ์ user openemr
GRANT ALL PRIVILEGES ON openemr_sandbox.* TO 'openemr'@'localhost';
FLUSH PRIVILEGES;
```

### Step 3: แก้ไข Config ให้ชี้ไป DB ใหม่
**ไฟล์:** `sites/sandbox/sqlconf.php`
```php
<?php
//  Eir — Sandbox Environment
//  MySQL Config

$host   = 'localhost';
$port   = '3306';
$login  = 'openemr';
$pass   = 'openemr';
$dbase  = 'openemr_sandbox';  // ← ชี้ไป DB สำหรับทดสอบ

$sqlconf = [];
global $sqlconf;
$sqlconf["host"]  = $host;
$sqlconf["port"]  = $port;
$sqlconf["login"] = $login;
$sqlconf["pass"]  = $pass;
$sqlconf["dbase"] = $dbase;

$config = 1; // ← เปลี่ยนเป็น 1 (ติดตั้งแล้ว)
```

### Step 4: รัน OpenEMR Setup Wizard สำหรับ Sandbox
```
เข้า URL: http://localhost:8300/interface/setup/setup.php?site=sandbox
→ ติดตั้ง Schema ลง DB openemr_sandbox
→ สร้าง Admin account สำหรับ Sandbox
```

---

## 3. Mock Data สำหรับ Sandbox

ข้อมูลจำลองด้านล่างนี้ออกแบบมาให้ **โครงสร้างเหมือน Data จริง 100%** แต่ใช้ชื่อ/ข้อมูลสมมติ เพื่อทดสอบ Migration Pipeline อย่างปลอดภัย

### 3.1 Mock Patients (5 คนไข้สมมติ ครอบคลุมทุกสถานะ)

```sql
-- ================================================
-- SANDBOX SEED DATA: ข้อมูลคนไข้จำลอง
-- รันบน DB: openemr_sandbox หลังติดตั้ง OpenEMR เสร็จ
-- ================================================

-- คนไข้ 1: สถานะปกติ (Triage Green)
INSERT INTO patient_data (fname, lname, DOB, sex, phone_home, email, pubpid, title)
VALUES ('สมชาย', 'ทดสอบ', '1975-03-15', 'Male', '081-111-1111', 'somchai@test.com', 'SANDBOX-PT-001', 'Mr.');

-- คนไข้ 2: สถานะเฝ้าระวัง (Triage Yellow)
INSERT INTO patient_data (fname, lname, DOB, sex, phone_home, email, pubpid, title)
VALUES ('สมหญิง', 'ทดสอบ', '1982-07-20', 'Female', '081-222-2222', 'somying@test.com', 'SANDBOX-PT-002', 'Ms.');

-- คนไข้ 3: สถานะวิกฤต (Triage Red — AHI > 30)
INSERT INTO patient_data (fname, lname, DOB, sex, phone_home, email, pubpid, title)
VALUES ('วิกฤต', 'ทดสอบ', '1968-11-03', 'Male', '081-333-3333', 'wikrit@test.com', 'SANDBOX-PT-003', 'Mr.');

-- คนไข้ 4: ไม่ให้ Consent (ทดสอบ Consent Gate)
INSERT INTO patient_data (fname, lname, DOB, sex, phone_home, email, pubpid, title)
VALUES ('ไม่ยินยอม', 'ทดสอบ', '1990-01-01', 'Female', '081-444-4444', 'noconsent@test.com', 'SANDBOX-PT-004', 'Mrs.');

-- คนไข้ 5: คนไข้ใหม่ (ยังไม่มีข้อมูล CPAP)
INSERT INTO patient_data (fname, lname, DOB, sex, phone_home, email, pubpid, title)
VALUES ('คนไข้ใหม่', 'ทดสอบ', '1995-06-10', 'Male', '081-555-5555', 'newpatient@test.com', 'SANDBOX-PT-005', 'Mr.');
```

### 3.2 Mock Data ฝั่ง Mega Care (Firestore Emulator)
ใช้คู่กับ Seed Data ด้านบน เพื่อทดสอบ Pipeline ทั้งเส้น

```json
[
  {
    "_doc_id": "SANDBOX-PT-001",
    "_description": "คนไข้ปกติ — Consent Active, Triage Green",
    "patient_list": {
      "airviewId": "SANDBOX-PT-001",
      "name": "ทดสอบ, สมชาย",
      "shouldExtract": true,
      "consent": { "status": "active", "version": "1.0" }
    },
    "prescription": {
      "device": { "name": "AirSense 10 AutoSet", "serialNumber": "SB-22001" },
      "settings": { "mode": "AutoSet", "pressure": { "min": 6.0, "max": 14.0 }, "epr": { "level": 2 } },
      "mask": { "name": "AirFit N20", "size": "Medium" }
    },
    "daily_report": {
      "reportDate": "2026-03-21",
      "statistics": {
        "usage": { "totalUsageSeconds": 27000 },
        "eventsPerHour": { "ahi": 2.1 },
        "leak": { "ninetyFifthPercentileLMin": 8.5 }
      },
      "triage_status": "Green"
    }
  },
  {
    "_doc_id": "SANDBOX-PT-002",
    "_description": "คนไข้เฝ้าระวัง — Consent Active, Triage Yellow",
    "patient_list": {
      "airviewId": "SANDBOX-PT-002",
      "name": "ทดสอบ, สมหญิง",
      "shouldExtract": true,
      "consent": { "status": "active", "version": "1.0" }
    },
    "prescription": {
      "device": { "name": "AirSense 11 AutoSet", "serialNumber": "SB-22002" },
      "settings": { "mode": "APAP", "pressure": { "min": 8.0, "max": 20.0 }, "epr": { "level": 3 } },
      "mask": { "name": "AirFit F20", "size": "Small" }
    },
    "daily_report": {
      "reportDate": "2026-03-21",
      "statistics": {
        "usage": { "totalUsageSeconds": 14400 },
        "eventsPerHour": { "ahi": 12.5 },
        "leak": { "ninetyFifthPercentileLMin": 28.0 }
      },
      "triage_status": "Yellow"
    }
  },
  {
    "_doc_id": "SANDBOX-PT-003",
    "_description": "คนไข้วิกฤต — Consent Active, Triage Red (AHI > 30)",
    "patient_list": {
      "airviewId": "SANDBOX-PT-003",
      "name": "ทดสอบ, วิกฤต",
      "shouldExtract": true,
      "consent": { "status": "active", "version": "1.0" }
    },
    "prescription": {
      "device": { "name": "AirSense 10 Elite", "serialNumber": "SB-22003" },
      "settings": { "mode": "CPAP", "pressure": { "min": 12.0, "max": 12.0 }, "epr": { "level": 1 } },
      "mask": { "name": "AirFit P10", "size": "Standard" }
    },
    "daily_report": {
      "reportDate": "2026-03-21",
      "statistics": {
        "usage": { "totalUsageSeconds": 10800 },
        "eventsPerHour": { "ahi": 42.3 },
        "leak": { "ninetyFifthPercentileLMin": 45.0 }
      },
      "triage_status": "Red"
    }
  },
  {
    "_doc_id": "SANDBOX-PT-004",
    "_description": "คนไข้ไม่ยินยอม — Consent Declined (ทดสอบ Consent Gate)",
    "patient_list": {
      "airviewId": "SANDBOX-PT-004",
      "name": "ทดสอบ, ไม่ยินยอม",
      "shouldExtract": false,
      "consent": { "status": "declined", "version": "1.0" }
    },
    "prescription": null,
    "daily_report": null
  },
  {
    "_doc_id": "SANDBOX-PT-005",
    "_description": "คนไข้ใหม่ — Consent Pending (ยังไม่ตอบ)",
    "patient_list": {
      "airviewId": "SANDBOX-PT-005",
      "name": "ทดสอบ, คนไข้ใหม่",
      "shouldExtract": false,
      "consent": { "status": "pending", "version": "0" }
    },
    "prescription": null,
    "daily_report": null
  }
]
```

---

## 4. Test Scenarios ที่ต้องรันบน Sandbox

| # | Test Case | Input (Mega Care Mock) | Expected (Eir Sandbox) | ทดสอบอะไร |
|---|-----------|------------------------|------------------------|-----------|
| T1 | Dual-Write: คนไข้ Green | SANDBOX-PT-001 + daily_report | สร้าง LBF Sleep Report สำเร็จ, AHI = 2.1 | ท่อข้อมูลปกติ |
| T2 | Dual-Write: คนไข้ Red | SANDBOX-PT-003 + daily_report | สร้าง LBF + ระบบ Flag "Urgent" | Triage Escalation |
| T3 | Consent Gate: Declined | SANDBOX-PT-004 | ❌ ไม่ถูก INSERT เข้า Eir | Consent Check |
| T4 | Consent Gate: Pending | SANDBOX-PT-005 | ❌ ไม่ถูก INSERT เข้า Eir | Consent Check |
| T5 | Backfill Script | ทุก Patient + ข้อมูลย้อนหลัง 30 วัน | Eir มีข้อมูลครบ 30 records ต่อคน | Data Integrity |
| T6 | Reconciliation Report | COUNT ทุก Collection | X (Mega Care) = Y (Eir) สำหรับ consent=active | จำนวนครบ |
| T7 | Idempotency Test | ยิง Dual-Write ซ้ำ 2 ครั้ง | ไม่เกิด Duplicate Record ใน Eir | ป้องกันข้อมูลซ้ำ |
| T8 | Appointment Sync | สร้างนัดหมาย Telemed + Google Meet Link | นัดหมายขึ้นใน Eir Calendar | Telemed Flow |

---

## 5. กฎเหล็ก: แยก Production กับ Sandbox

| กฎ | รายละเอียด |
|----|-----------|
| 🔴 **ห้าม** รัน Migration Script ชี้ตรงไป Production | ใช้ Environment Variable `EIR_SITE=sandbox` เสมอ |
| 🔴 **ห้าม** ใส่ข้อมูลจริงลง Sandbox | ใช้ Mock Data เท่านั้น (ชื่อ "ทดสอบ", ID "SANDBOX-PT-XXX") |
| 🟢 Production ให้ใช้ `?site=default` เท่านั้น | URL ปกติไม่ต้องใส่ query string |
| 🟢 Sandbox ต้องใส่ `?site=sandbox` เสมอ | เพื่อป้องกันเขียนข้อมูลผิด DB |
| 🟢 Mock Data ตั้งชื่อขึ้นต้นด้วย `SANDBOX-` | เห็นปุ๊บรู้ทันทีว่าเป็นข้อมูลจำลอง |

---

## 6. Integration กับ Sprint 5 Timeline

| วัน | งาน Sandbox | งาน Production |
|-----|-------------|----------------|
| **23 มี.ค.** | สร้าง `sites/sandbox/` + ติดตั้ง DB | ห้ามแตะ Production |
| **24 มี.ค.** | Seed Mock Data (SQL + Firestore Emulator) | สร้าง LBF Forms บน Production (ยังไม่มีข้อมูล) |
| **25 มี.ค.** | **รัน Test T1-T8 ทั้งหมดบน Sandbox** | ห้ามแตะ Production |
| **26 มี.ค.** | ✅ Sandbox ผ่านทุก Test | Deploy Consent Gate ขึ้น Production |

---

**Prepared By:** AI Systems Engineering Team

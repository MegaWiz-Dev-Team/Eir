# Migration Safety Review & Archive Plan
## การย้ายข้อมูลคนไข้จาก Mega Care → Eir (OpenEMR) อย่างปลอดภัย 100%

---

## ส่วนที่ 1: Safety Review — ตรวจสอบแผน Migration เดิม

### ✅ จุดที่แผนเดิมทำถูกต้องแล้ว
| # | หัวข้อ | ประเมิน |
|---|--------|---------|
| 1 | Firestore PITR + Manual Snapshot ก่อนเริ่ม | ✅ ครบ |
| 2 | Dry Run บน `mega-care-dev` ก่อนขึ้น Production | ✅ ครบ |
| 3 | Dual-Write แบบ Zero-Downtime | ✅ ออกแบบดี |
| 4 | Rollback Strategy กลับไป Mega Care ได้ | ✅ มีขั้นตอนชัดเจน |

### 🔴 จุดที่พบความเสี่ยง (Safety Gaps)

#### Gap 1: ไม่มีขั้นตอน "Reconciliation Check" (ตรวจยอดความครบถ้วน)
*   **ความเสี่ยง:** แผนเดิมมีแค่ Checksum/Hash ระดับ Record แต่ไม่มีขั้นตอนตรวจจำนวน "ทั้งหมด" → อาจ Migrate ไป 980 จาก 1,000 คน โดยไม่รู้ว่าหล่น 20 คน
*   **แก้ไข:** เพิ่มขั้นตอน **Post-Migration Reconciliation Report**:
    ```
    1. COUNT(*) patient_list ฝั่ง Mega Care  = X
    2. COUNT(*) patient ฝั่ง Eir             = Y
    3. ถ้า X ≠ Y → หยุดทันที, ดึง diff list ออกมาตรวจ
    4. ตรวจเพิ่ม: COUNT daily_reports, COUNT prescriptions แยกต่างหาก
    ```

#### Gap 2: ไม่มีกลไกป้องกัน "ข้อมูลซ้ำ" (Idempotency)
*   **ความเสี่ยง:** ถ้า Dual-Write ยิงซ้ำ (เช่น Cloud Function Retry) จะเกิด Duplicate Record ใน Eir
*   **แก้ไข:** ใช้ **Idempotency Key** = `{patient_id}_{report_date}` เป็น Unique Constraint ใน OpenEMR เพื่อป้องกัน INSERT ซ้ำ (ถ้ามี key ตรงกันอยู่แล้ว → UPDATE แทน)

#### Gap 3: ไม่มี Migration Log Table (บันทึกประวัติการย้าย)
*   **ความเสี่ยง:** ถ้า Script พังกลางทาง ไม่รู้ว่าย้ายไปถึง Record ไหนแล้ว ต้องเริ่มใหม่ตั้งแต่ต้น
*   **แก้ไข:** สร้างตาราง `migration_log` ทั้งใน Mega Care และ Eir:
    ```sql
    CREATE TABLE migration_log (
      id              INT AUTO_INCREMENT PRIMARY KEY,
      patient_id      VARCHAR(255) NOT NULL,
      data_type       ENUM('demographics','prescription','daily_report','compliance_report','appointment') NOT NULL,
      source_doc_id   VARCHAR(255),
      status          ENUM('pending','success','failed','skipped') DEFAULT 'pending',
      error_message   TEXT,
      migrated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      UNIQUE KEY unique_migration (patient_id, data_type, source_doc_id)
    );
    ```

#### Gap 4: ไม่มีการเข้ารหัสข้อมูลระหว่างส่ง (Encryption in Transit)
*   **ความเสี่ยง:** ข้อมูลที่วิ่งจาก Cloud Function (Mega Care) ไป Eir API ถ้าไม่บังคับ HTTPS + มี mTLS อาจถูกดักสนิฟได้
*   **แก้ไข:** บังคับ Eir API ใช้ HTTPS Only + ตรวจสอบ SSL Certificate ฝั่ง Client ด้วย

#### Gap 5: ไม่มี "Consent Check Gate" ก่อน Migrate
*   **ความเสี่ยง:** ถ้าคนไข้ Revoke Consent (ถอนความยินยอม PDPA) แล้ว แต่ Script ยังดันข้อมูลไป Eir จะถือว่าละเมิด PDPA
*   **แก้ไข:** ก่อนเขียนข้อมูลแต่ละ Record เข้า Eir ดึงค่า `consent.status` มาเช็คก่อน ถ้า ≠ `"active"` → `SKIP` และเขียนลง `migration_log` สถานะ `"skipped"`

---

## ส่วนที่ 2: แผน Migration ฉบับปรับปรุง (5 Phases)

### Phase 1: Pre-Flight (ก่อนลงมือ)
| ลำดับ | งาน | ผู้รับผิดชอบ | เสร็จภายใน |
|-------|------|-------------|-----------|
| 1.1 | เปิด PITR บน Firestore (Production) | DevOps | Day 1 |
| 1.2 | รัน `gcloud firestore export` → GCS Coldline | DevOps | Day 1 |
| 1.3 | สร้าง BigQuery Snapshot Tables | DevOps | Day 1 |
| 1.4 | สร้าง `migration_log` table ใน Eir (MySQL) | Dev Eir | Day 1-2 |
| 1.5 | สร้าง LBF Forms (CPAP Prescription + Sleep Report) ใน Eir | Dev Eir | Day 1-3 |
| 1.6 | สร้าง Custom Inbound API (`POST /cpap-sync`) ใน Eir | Dev Eir | Day 3-5 |
| 1.7 | Dry Run บน `mega-care-dev` → Eir Staging | ทั้ง 2 ทีม | Day 5-7 |

### Phase 2: Dual-Write (คู่ขนาน 2 สัปดาห์)
| ลำดับ | งาน | เกณฑ์ผ่าน |
|-------|------|----------|
| 2.1 | แก้ Cloud Functions เขียนลงทั้ง Mega Care + Eir | ไม่มี Error ใน Cloud Logging 48 ชม. |
| 2.2 | ตรวจ Reconciliation Report ทุกเช้า | COUNT ต้องตรงกัน 100% |
| 2.3 | หมอ/พยาบาลทดสอบดูข้อมูลใน Eir | Feedback OK จากผู้ใช้จริง |

### Phase 3: Backfill (กวาดข้อมูลย้อนหลัง)
| ลำดับ | งาน | เกณฑ์ผ่าน |
|-------|------|----------|
| 3.1 | รัน Migration Script (Batch, ตี 2) | `migration_log` status = `success` ทั้งหมด |
| 3.2 | Consent Check Gate ก่อน INSERT ทุก Record | `skipped` = เฉพาะ Revoked Consent เท่านั้น |
| 3.3 | Post-Migration Reconciliation Report | X = Y สำหรับทุก Collection |

### Phase 4: Cut-Over (สับสวิตช์)
| ลำดับ | งาน | เกณฑ์ผ่าน |
|-------|------|----------|
| 4.1 | Frontend Mega Care อ่าน Clinical Data จาก Eir API | Dashboard ยังแสดงผลปกติ |
| 4.2 | หยุด Write Clinical Data ลง Firestore | ข้อมูลใหม่ไหลเข้า Eir เท่านั้น |
| 4.3 | Monitoring 1 สัปดาห์ | ไม่มี Error / ข้อมูลหล่น |

### Phase 5: Archive (เก็บกวาดข้อมูลเก่า) → ดูรายละเอียดในส่วนที่ 3

---

## ส่วนที่ 3: แผน Archive ข้อมูลบน Mega Care Portal

> **เป้าหมาย:** ลบข้อมูลทางคลินิก (Clinical Data) ออกจาก Mega Care Firestore เพื่อลดพื้นที่โจมตี (Attack Surface) และลดโอกาสข้อมูลรั่วไหล ในขณะที่ยังเก็บสำเนาสำรองไว้ตามกฎหมาย

### 3.1 ข้อมูลที่ต้อง Archive (ย้ายออกจาก Firestore)
| Collection | เหตุผล | ปลายทาง Archive |
|------------|--------|-----------------|
| `prescriptions` (Subcollection) | เป็นคำสั่งแพทย์ Source of Truth อยู่ Eir แล้ว | GCS Coldline + BigQuery Archive Dataset |
| `daily_reports` (Subcollection) | เป็นข้อมูลวินิจฉัย Source of Truth อยู่ Eir แล้ว | GCS Coldline + BigQuery Archive Dataset |
| `compliance_reports` (Subcollection) | เป็นข้อมูลวินิจฉัย Source of Truth อยู่ Eir แล้ว | GCS Coldline + BigQuery Archive Dataset |
| PDF Reports ใน GCS | ไฟล์ต้นฉบับย้ายเข้า Eir Documents แล้ว | GCS Archive Bucket (แยก Bucket) |

### 3.2 ข้อมูลที่ต้องเก็บไว้ใน Mega Care ต่อ (ห้ามลบ)
| Collection | เหตุผล |
|------------|--------|
| `patient_list` | ยังใช้สำหรับ CRM, Logisitics, และ Triage Dashboard |
| `patient_details` (เฉพาะ Operational Info) | ข้อมูลสาขา, องค์กร, สถานะ Setup |
| `devices` | Asset Tracking ของเครื่อง CPAP |
| `todos` | งานแอดมินประจำวัน |
| `extraction_jobs` | ประวัติ Job สำหรับ Audit Trail |
| `telemed_appointments` | ยังใช้จัดการนัดหมายร่วมกับ Eir |
| `triage_status` (field ใน patient_list) | สถานะไฟเขียว/เหลือง/แดง สำหรับ Ops |

### 3.3 ขั้นตอนการ Archive อย่างปลอดภัย (Safety Protocol)

```
Step 1: PRE-ARCHIVE VERIFICATION
   ├── ยืนยันว่า Phase 4 (Cut-Over) สำเร็จแล้ว
   ├── ยืนยัน Reconciliation Report ตรง 100%
   └── ยืนยันว่าไม่มี Frontend ใดๆ ยังอ่าน Clinical Data จาก Firestore โดยตรง

Step 2: EXPORT TO ARCHIVE
   ├── Export Firestore Collections → GCS (Coldline)
   │   └── Bucket: gs://mega-care-clinical-archive/YYYY-MM-DD/
   │       ├── prescriptions/
   │       ├── daily_reports/
   │       └── compliance_reports/
   ├── ตั้ง Retention Policy: ล็อกห้ามลบ 10 ปี (ตาม พ.ร.บ. สถานพยาบาล)
   └── ตั้ง Access Control: เฉพาะ DPO (Data Protection Officer) เท่านั้น

Step 3: SOFT DELETE FROM FIRESTORE
   ├── ลบ Subcollections: daily_reports, compliance_reports, prescriptions
   ├── อัปเดต patient_list: เพิ่ม field `clinical_data_archived: true`
   └── ลบ field `triage_status` ที่แนบข้อมูลวินิจฉัยเชิงลึก (เก็บแค่ status สี)

Step 4: CLEANUP BIGQUERY
   ├── ย้าย Table patient_reports → Dataset: analytics_archive
   ├── ลบ Staging Tables
   └── เก็บ patients_master_view ไว้ (ใช้สำหรับ Dashboard Ops ต่อ)

Step 5: POST-ARCHIVE VERIFICATION
   ├── หมอ/พยาบาล ทดสอบเปิดดูข้อมูลใน Eir → ข้อมูลครบ
   ├── แอดมิน ทดสอบใช้ Mega Care Dashboard → Dashboard ไม่พัง
   └── ทดสอบ Mega Care ไม่เห็นข้อมูล Clinical อีก → ✅ ลดความเสี่ยงรั่วไหล
```

### 3.4 กฎหมายที่ต้องปฏิบัติตาม (Retention Policy)
| กฎหมาย / มาตรฐาน | ข้อกำหนด | การปฏิบัติ |
|-------------------|----------|-----------|
| **PDPA ม.37(3)** | ลบข้อมูลเมื่อหมดวัตถุประสงค์ | ลบ Clinical Data ออกจาก Mega Care (ส่ง Eir แทน) |
| **พ.ร.บ. สถานพยาบาล** | เก็บเวชระเบียน ≥ 5 ปี | Archive ใน GCS Coldline + Eir 10 ปี |
| **HIPAA** | Data at Rest ต้องเข้ารหัส | GCS + Eir MySQL = Encrypted by Default |
| **ISO 27001** | ต้องมี Audit Trail | `migration_log` + `extraction_jobs` + Cloud Logging |

---

## ส่วนที่ 4: Rollback Strategy ฉบับปรับปรุง

| สถานการณ์ | การตอบสนอง | RTO | RPO |
|-----------|-----------|-----|-----|
| Dual-Write ล้มเหลว (Phase 2) | หยุด Write ไป Eir, Mega Care ยังทำงานปกติ | 0 นาที | 0 (ข้อมูลยังอยู่ Mega Care) |
| Backfill Script พัง (Phase 3) | ดู `migration_log` หาจุดที่หยุด, รันต่อจากจุดนั้น | 30 นาที | 0 |
| Cut-Over แล้วแต่ Eir ล่ม (Phase 4) | สับสวิตช์ Frontend กลับอ่าน Mega Care | 15 นาที | < 1 ชม. |
| Archive แล้วแต่พบปัญหา (Phase 5) | Restore จาก GCS Coldline กลับเข้า Firestore | 2-4 ชม. | 0 (ข้อมูล Archive ครบ) |

---

**Prepared By:** AI Systems Engineering Team
**Review Date:** 2026-03-22

# PDPA Consent Remediation Plan
## แผนการจัดการปัญหา "ยังไม่ได้ขอ Consent คนไข้" บน Mega Care Admin Portal

---

## สถานะปัจจุบัน (Current State Analysis)

### ✅ สิ่งที่มีแล้วในระบบ (จากการตรวจ Codebase จริง)
| # | รายการ | ไฟล์ | สถานะ |
|---|--------|------|-------|
| 1 | API อัปเดต Consent: `PUT /{patient_id}/consent` | `backend/app/api/v1/endpoints/patients.py` L212-254 | ✅ ใช้งานได้ |
| 2 | API ลบข้อมูล (Right to be Forgotten) | `backend/app/api/v1/endpoints/patients.py` L334-361 | ✅ ใช้งานได้ |
| 3 | แสดงสถานะ Consent บน Triage Dashboard | `frontend/.../TriageDashboardPage.js` L913, L936 | ✅ แสดงผลอยู่ |
| 4 | ฟิลด์ `consent` ใน Firestore (`patient_list`) | `technical-spec.md` Section 2 | ✅ มีโครงสร้าง |

### 🔴 สิ่งที่ยังขาด (Gaps ที่ต้องอุด)
| # | รายการ | ความเสี่ยง | ความเร่งด่วน |
|---|--------|-----------|-------------|
| 1 | **ไม่มี Consent Gate ใน Extraction Agent** — Agent ดูดข้อมูลจาก AirView ทุกคนโดยไม่เช็ค consent | ละเมิด PDPA ม.26 | 🔴 ด่วนมาก |
| 2 | **ไม่มี e-Consent Flow สำหรับคนไข้** — ไม่มีหน้าจอให้คนไข้กดยินยอม | ไม่มีหลักฐาน consent | 🔴 ด่วนมาก |
| 3 | **ไม่มี Consent Gate ใน Cloud Functions** — `processReportUpload` ประมวลผล Report ทุก Record | ประมวลผลข้อมูลคนที่ไม่ยินยอม | 🟡 ด่วน |
| 4 | **ไม่มี Consent Audit Log** — ไม่บันทึกว่าใครให้/ถอน consent เมื่อไหร่ | พิสูจน์ไม่ได้หากถูกตรวจสอบ | 🟡 ด่วน |
| 5 | **คนไข้เก่าทั้งหมดไม่มี consent** — ข้อมูลในระบบทั้งหมดไม่เคยขอ consent | ละเมิด PDPA ทั้งฐาน | 🔴 ด่วนมาก |

---

## แผนการดำเนินงาน 4 ระยะ

### 🚨 ระยะที่ 1: Risk Mitigation ทันที (สัปดาห์ที่ 1)

**เป้าหมาย:** ลดความเสี่ยงทางกฎหมายทันทีโดยยังไม่ต้องหยุดระบบ

#### 1.1 ล็อกการเข้าถึง (Minimum Access)
*   Review รายชื่อ Admin ทั้งหมดที่เข้า Portal ลดให้เหลือเฉพาะคนที่จำเป็น
*   ลบ Account ที่ไม่ใช้งานแล้วออกจาก Firebase Auth ทันที

#### 1.2 เพิ่ม Consent Check Gate ในโค้ด
**ไฟล์ที่ต้องแก้ 3 จุด:**

**จุดที่ 1: Extraction Agent** (`agent/`)
```python
# ก่อนดึงข้อมูลจาก AirView ให้เช็ค consent ก่อน
patient_doc = db.collection('patient_list').document(patient_id).get()
consent = patient_doc.to_dict().get('consent', {})
if consent.get('status') != 'active':
    logger.warning(f"SKIP {patient_id}: consent not active ({consent.get('status')})")
    continue  # ข้ามคนไข้คนนี้
```

**จุดที่ 2: Cloud Function `processReportUpload`** (`cloud-functions/`)
```python
# ก่อนประมวลผล Report ด้วย AI ให้เช็ค consent ก่อน
patient_ref = db.collection('patient_list').document(patient_id)
patient = patient_ref.get().to_dict()
if (patient.get('consent', {}).get('status') or 'pending') != 'active':
    logger.info(f"SKIP processing report for {patient_id}: no active consent")
    return  # ไม่ประมวลผล → ไม่สร้าง daily_report → ไม่ส่ง BigQuery
```

**จุดที่ 3: Daily Extraction Trigger** (`daily-patient-extraction-trigger/`)
```python
# ใน Query ที่ดึงรายชื่อคนไข้ "Today" ให้กรองเฉพาะ consent active
query = db.collection('patient_list') \
    .where('lastUpdatedSourceText', '==', 'Today') \
    .where('consent.status', '==', 'active')  # ← เพิ่มบรรทัดนี้
```

#### 1.3 ตั้งค่าเริ่มต้นสำหรับคนไข้ทั้งหมดที่ยังไม่มี consent
รัน Script ครั้งเดียวเพื่ออัปเดตคนไข้ทุกคนที่ยังไม่มี field `consent`:
```python
patients = db.collection('patient_list').stream()
for patient in patients:
    data = patient.to_dict()
    if 'consent' not in data:
        patient.reference.update({
            'consent': {
                'status': 'pending',       # ← ยังไม่ได้ขอ
                'version': '0',
                'last_updated': firestore.SERVER_TIMESTAMP
            }
        })
```
> ⚠️ **สำคัญ:** ตั้งเป็น `"pending"` ไม่ใช่ `"active"` เพราะยังไม่ได้ขอจริง

---

### 📋 ระยะที่ 2: สร้างระบบ e-Consent (สัปดาห์ที่ 2-3)

**เป้าหมาย:** สร้างกลไกให้คนไข้กดยินยอมด้วยตัวเอง

#### 2.1 ช่องทางการขอ Consent

| ช่องทาง | กลุ่มเป้าหมาย | วิธีทำ |
|---------|--------------|--------|
| **Line OA** | คนไข้เก่าที่มี Line ID อยู่แล้ว | ส่ง Flex Message พร้อมปุ่ม "ยินยอม / ไม่ยินยอม" |
| **SMS** | คนไข้เก่าที่ไม่มี Line ID | ส่ง SMS Short Link ไปหน้า consent form |
| **Mega Care Connect (Web)** | คนไข้ใหม่ทุกคน | บังคับกดติ๊ก consent ก่อน submit ลงทะเบียน |
| **Admin Portal (Manual)** | กรณีโทรถาม consent ทางโทรศัพท์ | Admin กดปุ่มบันทึก consent ด้วยตัวเอง (มี Audit Log) |

#### 2.2 เนื้อหา Consent Form (ภาษาที่ถูกต้องตาม PDPA)

```
═══════════════════════════════════════════════
 แบบฟอร์มให้ความยินยอมในการประมวลผลข้อมูลสุขภาพ
 (Health Data Consent Form - PDPA Compliant)
═══════════════════════════════════════════════

ข้าพเจ้า [ชื่อ-นามสกุล] ยินยอมให้ บริษัท เมกาวิซ จำกัด
ดำเนินการประมวลผลข้อมูลสุขภาพของข้าพเจ้า ดังต่อไปนี้:

📋 ข้อมูลที่จะประมวลผล:
  • ข้อมูลการใช้เครื่อง CPAP (ชั่วโมงการใช้, ค่าลมรั่ว)
  • ดัชนีการหยุดหายใจขณะหลับ (AHI)
  • รายงานสุขภาพการนอนรายวัน

🎯 วัตถุประสงค์:
  1. ติดตามการใช้เครื่องและดูแลสุขภาพจากระยะไกล
  2. แจ้งเตือนเมื่อพบค่าผิดปกติ
  3. ส่งต่อข้อมูลให้แพทย์ผู้เชี่ยวชาญเพื่อวินิจฉัยและรักษา
     ผ่านระบบเวชระเบียนอิเล็กทรอนิกส์ (Eir)

🔒 การคุ้มครอง:
  • ข้อมูลจะถูกเข้ารหัสและจัดเก็บอย่างปลอดภัย
  • เฉพาะบุคลากรที่ได้รับมอบหมายเท่านั้นที่สามารถเข้าถึงได้
  • ข้าพเจ้ามีสิทธิ์ขอดู แก้ไข หรือลบข้อมูลได้ตลอดเวลา

⚠️ ข้าพเจ้าสามารถถอนความยินยอมได้ตลอดเวลา
   โดยแจ้งผ่าน Line OA หรือโทร 02-XXX-XXXX

☐ ข้าพเจ้ายินยอม (Consent)
☐ ข้าพเจ้าไม่ยินยอม (Decline)

วันที่ให้ความยินยอม: [Auto-fill Timestamp]
เวอร์ชันเอกสาร: v1.0
```

#### 2.3 Consent Data Schema (บันทึกลง Firestore)

```json
{
  "consent": {
    "status": "active",
    "version": "1.0",
    "channel": "line_oa",
    "consented_at": "2026-03-25T10:30:00Z",
    "ip_address": "203.150.xxx.xxx",
    "last_updated": "2026-03-25T10:30:00Z"
  }
}
```

---

### 📞 ระยะที่ 3: Outreach Campaign — ขอ Consent คนไข้เก่า (สัปดาห์ที่ 3-5)

#### 3.1 แผนการส่ง Consent Request

```
สัปดาห์ที่ 3: ส่ง Batch 1   → คนไข้ที่มี Line ID (ส่งผ่าน Line OA)
สัปดาห์ที่ 4: Reminder #1   → คนที่ยังไม่ตอบ + Batch 2 (SMS กลุ่มที่ไม่มี Line)
สัปดาห์ที่ 5: Reminder #2   → Final Reminder ทุกช่องทาง
```

#### 3.2 การจัดการผลลัพธ์

| ผลลัพธ์ | การดำเนินการทาง Technical |
|---------|--------------------------|
| **✅ ยินยอม** | `consent.status = "active"` → ดำเนินการดึงข้อมูล + Migrate → Eir ต่อได้ |
| **❌ ไม่ยินยอม** | `consent.status = "declined"` + `shouldExtract = false` → หยุดดึงข้อมูล |
| **⏳ ไม่ตอบ (หลัง 2 Reminders)** | `consent.status = "expired"` + `shouldExtract = false` → หยุดดึงข้อมูล, เก็บข้อมูลเดิมไว้ 90 วัน แล้ว Anonymize |

#### 3.3 Dashboard ติดตามผล Consent Campaign

เพิ่มหน้าจอใหม่บน Admin Portal:
*   **Consent Overview:** กราฟ Pie Chart แสดง % Active / Pending / Declined / Expired
*   **Actionable List:** รายชื่อคนไข้ที่ Consent ยังเป็น "pending" เรียงตามวันที่ส่ง Consent Request
*   **Export:** ส่งออก CSV รายชื่อเพื่อใช้ส่ง Reminder ผ่าน Line OA / SMS

---

### 🔒 ระยะที่ 4: บังคับ Consent ในทุกจุดเชื่อมต่อ (ระยะยาว)

#### 4.1 Consent Gate ทุกจุดที่ข้อมูลไหลผ่าน

```
                                    ┌─── Consent Gate #1
                                    │    (ก่อนดึงจาก AirView)
                                    ▼
AirView  ──→  Extraction Agent  ──→  Firestore
                                         │
                                    ┌────┘
                                    │    Consent Gate #2
                                    │    (ก่อนประมวลผล Report)
                                    ▼
                              Cloud Function  ──→  BigQuery
                                         │
                                    ┌────┘
                                    │    Consent Gate #3
                                    │    (ก่อนส่งข้อมูล → Eir)
                                    ▼
                              Dual-Write API  ──→  Eir (OpenEMR)
```

#### 4.2 หน้าจอ Consent สำหรับคนไข้ใหม่ (Mega Care Connect)
*   หน้าลงทะเบียนบังคับติ๊ก consent ก่อน Submit
*   ไม่สามารถข้ามได้ (Backend ต้องเช็คด้วย ไม่ใช่แค่ Frontend)

#### 4.3 Consent Versioning & Re-Consent
*   เมื่อเนื้อหา consent เปลี่ยน (เช่น เพิ่มวัตถุประสงค์ใหม่) → อัป version ให้เป็น v2.0
*   คนไข้ที่เคยยินยอม v1.0 ต้องได้รับแจ้งเตือนให้กดยินยอม v2.0 ใหม่
*   ถ้ายังไม่ยินยอม v2.0 → ใช้ได้แค่ฟีเจอร์ที่ v1.0 ครอบคลุม

---

## Timeline สรุป

```
สัปดาห์ที่ 1  │ 🚨 Risk Mitigation
              │  ├── ล็อกสิทธิ์ Admin
              │  ├── เพิ่ม Consent Gate 3 จุดในโค้ด
              │  └── ตั้ง consent.status = "pending" ให้คนไข้เก่าทั้งหมด
              │
สัปดาห์ที่ 2-3│ 📋 สร้างระบบ e-Consent
              │  ├── พัฒนา Consent Form (Line OA + Web)
              │  ├── พัฒนา Consent Dashboard
              │  └── ทดสอบ Flow ให้/ถอน consent
              │
สัปดาห์ที่ 3-5│ 📞 Outreach Campaign
              │  ├── ส่ง Consent Request Batch 1 (Line OA)
              │  ├── Reminder #1
              │  └── Reminder #2 + Final
              │
สัปดาห์ที่ 6+ │ 🔒 บังคับ Consent ถาวร
              │  ├── Consent Gate บังคับทุกจุด
              │  ├── คนไข้ใหม่ต้องยินยอมก่อนใช้บริการ
              │  └── Anonymize ข้อมูลคนที่ไม่ยินยอม/ไม่ตอบ
```

---

## ข้อแนะนำเพิ่มเติมสำหรับฝ่ายบริหาร
1.  **แต่งตั้ง DPO (Data Protection Officer)** หรือผู้รับผิดชอบ PDPA อย่างเป็นทางการ
2.  **จัดทำ ROPA (Record of Processing Activities)** บันทึกกิจกรรมการประมวลผลข้อมูลทั้งหมด
3.  **ปรึกษาทนายความด้าน PDPA** เพื่อ Review เนื้อหา Consent Form ก่อนส่งออกจริง
4.  **มี Incident Response Plan** — หากเกิดเหตุข้อมูลรั่วไหล ต้องแจ้ง PDPC ภายใน 72 ชั่วโมง

---

**Prepared By:** AI Systems Engineering Team
**Review Date:** 2026-03-22

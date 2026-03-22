# Data Schema Mapping: Mega Care Firestore to Eir (OpenEMR)

เอกสารฉบับนี้สรุปโครงสร้างข้อมูล (Data Structure) ของฐานข้อมูล `mega-care-db` บน Firestore และแผนการทำ Data Mapping เพื่อนำข้อมูลเข้าสู่ระบบเวชระเบียน Eir ที่มีพื้นฐานมาจากตัว **OpenEMR**

---

## 1. ข้อมูลผู้ป่วยพื้นฐาน (Patient Demographics)

ข้อมูลกลุ่มนี้จะถูกจับคู่ (Map) เข้ากับตารางมาตรฐานของ OpenEMR คือตาราง `patient_data`

| Mega Care Firestore (Source) | Data Type | Field in Eir / OpenEMR (`patient_data`) | Type (Target) | Remarks / Transformation |
| :--- | :--- | :--- | :--- | :--- |
| `patient_details.personalInfo.firstName` | String | `fname` | varchar(255) | สามารถอัปเดตตรงได้ทันที |
| `patient_details.personalInfo.lastName` | String | `lname` | varchar(255) | สามารถอัปเดตตรงได้ทันที |
| `patient_details.personalInfo.dob` | Timestamp | `DOB` | date | แปลงจาก Firestore Timestamp เป็น YYYY-MM-DD |
| `patient_details.personalInfo.title` | String | `title` | varchar(255) | Mr., Mrs., Ms. |
| `patient_details.personalInfo.phone` | String | `phone_home` / `phone_cell` | varchar(255) | เบอร์โทรศัพท์ผู้ป่วย |
| `patient_details.personalInfo.lineId` | String | `usertext1` (Custom) | varchar(255) | Line ID ผู้ป่วย (เพิ่มคอลัมน์ Custom หรือ LBF Demographics) |
| `patient_details.personalInfo.email` | String | `email` | varchar(255) | อีเมลผู้ป่วย |
| `patient_list.airviewId` | String | `pubpid` (หรือ `uuid`) | varchar(255) | ใช้เป็น Primary/External ID สำหรับผูกกับ AirView |
| `patient_list.consent.status` | String | `genericval1` (Custom) | varchar(255) | บันทึกสถานะ PDPA ตรงนี้ หรือใช้ตาราง `consent_docs` |

---

## 2. ข้อมูลอุปกรณ์และคำสั่งแพทย์ (Device & Prescriptions)

ระบบ OpenEMR ปกติจะเก็บใบสั่งยาเข้าตาราง `prescriptions` ทว่าข้อมูล CPAP เป็นการ "ตั้งค่าอุปกรณ์ทางการแพทย์" จึงแนะนำให้สร้าง **LBF (Layout-Based Form)** ชื่อ "CPAP_Prescription" มารองรับ

| Mega Care Firestore (Source) | Data Type | Field in Eir LBF (`form_cpap_prescription`) | Type (Target) | Remarks |
| :--- | :--- | :--- | :--- | :--- |
| `prescriptions.device.name` | String | `cpap_model` | Text | เช่น "AirSense 10 AutoSet" |
| `prescriptions.device.serialNumber`| String | `cpap_sn` | Text | |
| `prescriptions.settings.mode` | String | `therapy_mode` | Dropdown | CPAP, APAP, AutoSet |
| `prescriptions.settings.pressure.min`| Number | `pressure_min` | Float | หน่วย cmH2O |
| `prescriptions.settings.pressure.max`| Number | `pressure_max` | Float | หน่วย cmH2O |
| `prescriptions.settings.epr.level`| Number | `epr_level` | Int | 1, 2, 3 |
| `prescriptions.mask.name` | String | `mask_type` | Text | เช่น "AirFit N20" |

---

## 3. ข้อมูลดัชนีการนอนหลับ (Clinical Sleep Reports)

รายงานเช่น AHI, Leak และไฟล์ PDF จะถูก Map เข้าสู่ระบบ **Encounter / Documents** ใน OpenEMR 

### 3.1 ข้อมูลตัวเลข (เพื่อใช้สร้างกราฟ)
สร้างแบบฟอร์ม **LBF** ชื่อ "Sleep_Report_Data" ผูกกับ Encounter (การเข้าตรวจ/Telemed รายครั้ง)

| Mega Care Firestore (Source) | Data Type | Field in Eir LBF (`form_sleep_report`) | Type (Target) | Remarks |
| :--- | :--- | :--- | :--- | :--- |
| `daily_reports.reportDetails.reportDate`| Timestamp | `date` | Date | วันที่ของ Report |
| `daily_reports.statistics.usage.totalUsageSeconds` | Number | `usage_hours` | Float | คอนเวิร์ตวินาทีเป็นชั่วโมง |
| `daily_reports.statistics.eventsPerHour.ahi`| Number | `ahi` | Float | อัตราการหยุดหายใจ |
| `daily_reports.statistics.leak.ninetyFifthPercentileLMin`| Number | `leak_95th` | Float | ปริมาณลมรั่ว |
| `daily_reports.analysis_and_recommendations.patient_triage.triage_status`| String | `triage_status` | Text | Red, Yellow, Green (อ้างอิงจาก AI) |

### 3.2 ไฟล์เอกสาร PDF
*   **Mega Care:** `daily_reports.system.downloadUrl` หรือ `sourceStorageUri`
*   **Eir / OpenEMR:** จะต้องดึงไฟล์ PDF จาก GCS แล้วบันทึกลงใน **Documents Module** (`categories` = "Sleep Test Reports") โดยลิงก์กับ Patient ID ของคนไข้

---

## 4. โครงสร้าง JSON Payload ตัวอย่าง (สำหรับการทำ Inbound API เข้า Eir)

ฝั่ง Eir ระบบจะต้องมี REST API Endpoint (เช่น `POST /apis/default/api/clinical/sleep-reports`) เพื่อรับ Payload จาก Mega Care ในรูปแบบนี้:

```json
{
  "mega_care_patient_id": "ee319d58-9aeb-4af7-b156-f91540689595",
  "encounter_date": "2024-08-15",
  "prescription": {
    "mode": "AutoSet",
    "min_pressure": 7.0,
    "max_pressure": 17.0,
    "epr": 3
  },
  "sleep_data": {
    "ahi": 1.5,
    "leak": 12.0,
    "usage_hours": 7.5,
    "triage": "Green"
  },
  "report_url": "https://storage.googleapis.com/.../report.pdf"
}
```

---

## 5. ข้อมูลการนัดหมาย Telemedicine (Appointment Data)

สำหรับระบบนัดหมาย (จากสคีมา `TelemedAppointment` ของ Mega Care) จะ Map เข้ากับตาราง `openemr_postcalendar_events` ของ Eir

| Mega Care Firestore (Source) | Field in Eir (`postcalendar_events`) | Remarks |
| :--- | :--- | :--- |
| `telemed_appointments.date` | `pc_eventDate` | วันที่นัดหมาย (YYYY-MM-DD) |
| `telemed_appointments.time` | `pc_startTime` | เวลานัดหมาย (HH:MM:SS) |
| `telemed_appointments.duration`| `pc_duration` | ระยะเวลา (นาที) |
| `telemed_appointments.doctor_id`| `pc_aid` | ID ผู้ให้บริการ/แพทย์ประจำ Eir |
| `telemed_appointments.google_meet_link` | LBF / Custom Notes | นำลิงก์ไปแปะไว้ในช่อง Note ของนัดหมายนั้นๆ |
| `telemed_appointments.status` | `pc_apptstatus` | แปลงสถานะ Pending -> `< >`, Confirmed -> `@`, Completed -> `!` |

## แผนงานสำหรับทีม Eir (Next Steps)
1. เข้าไปที่ OpenEMR `Administration > Forms > Layouts` สร้าง LBF Form ชื่อ **CPAP Prescription** และ **Sleep Report** ให้มีฟิลด์ตรงตามตารางด้านบน
2. พัฒนา API `POST /cpap-sync` และ `POST /telemed-appointments` เพื่อรับข้อมูลจาก Mega Care ทะลุเข้า Eir
3. อัปเดตตาราง `patient_data` ให้รองรับ UUID จาก AirView/MegaCare และ Custom Field พวก Line ID

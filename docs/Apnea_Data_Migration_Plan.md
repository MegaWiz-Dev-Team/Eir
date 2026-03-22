# Implementation & Data Migration Plan: Mega Care to Eir

เอกสารฉบับนี้อธิบายแผนงานสำหรับการย้ายข้อมูลทางคลินิก (Clinical Data) จาก `mega-care-admin-portal` ไปยัง `Eir` อย่างปลอดภัย พร้อมแผนการวางระบบสำรองข้อมูล (Backup) และการอุดช่องโหว่ตาม Gap Analysis เพื่อให้ระบบสอดคล้องกับมาตรฐานทางด้านการแพทย์

---

## 🔒 1. แผนความปลอดภัยและการสำรองข้อมูล (Safety & Backup Plan)
ก่อนเริ่มกระบวนการ Migrate ใดๆ ต้องเตรียม Infrastructure ด้านความปลอดภัยให้พร้อม 100%

### 1.1 Pre-Migration Backup (ก่อนเริ่มย้ายข้อมูล)
*   **Firestore Point-in-Time Recovery (PITR):** ระบุให้มั่นใจว่าเปิดใช้งาน PITR บน Firestore ของ GCP Project `mega-care` (Production) แล้ว เพื่อให้สามารถสั่งย้อนเวลากลับ (Rollback) ได้ทุกระดับนาที (สูงสุด 7 วันย้อนหลัง) หากการ Migrate ผิดพลาด
*   **Manual Export (Snapshot):**
    *   รันคำสั่ง `gcloud firestore export` เพื่อแบ็กอัป Collection `prescriptions`, `daily_reports`, และ `compliance_reports` ทั้งหมดไปเก็บไว้ที่ Google Cloud Storage (GCS) แบบ Coldline Storage ล็อกไว้ห้ามลบ (Retention Policy ป้องกันการแก้ไข)
*   **BigQuery Snapshot:** ทำตาราง Snapshot (Table Copy) ของ `patient_reports` และ `patients_master_view` ปัจจุบันเก็บไว้เป็นชื่อ `patient_reports_backup_YYYYMMDD`

### 1.2 Data Integrity (ระหว่างย้ายข้อมูล)
*   **Dry Run & Staging:** ทดสอบสคริปต์การย้ายข้อมูล (Migration Script) บนฐานข้อมูลจำลองในสภาพแวดล้อม `mega-care-dev` ก่อนเสมอ
*   **Checksum/Hash Validation:** หลังการคัดลอก JSON payload ของใบสั่งยาและรีพอร์ตแต่ละฉบับ ให้เทียบ Hash ว่าจำนวน Byte และข้อมูล Record ของ Eir กับ Mega Care ตรงกันครบถ้วน

---

## 🔄 2. Data Migration Strategy (แผนการโอนย้ายข้อมูลคลินิก)
ใช้แนวทาง **Dual-Write + Data Backfill** เพื่อให้เกิด **Zero-Downtime** (ระบบไม่ต้องหยุดทำงานระหว่างย้าย)

### Phase 2.1: Schema Preparation (Eir)
1.  **สร้าง Database Schema ฝั่ง Eir:**
    *   สร้างตาราง/Collection มารองรับ Medical Order (Prescription) และ Sleep Analysis Reports
    *   เตรียมฟิลด์สำหรับเก็บ `mega_care_patient_id` เพื่อใช้เป็น UUID อ้างอิงโยงกลับมาที่ Mega Care

### Phase 2.2: The "Dual-Write" (เปิดท่อคู่ขนาน)
1.  แก้โค้ดที่ `Cloud Function` ฝั่ง Mega Care 
2.  เมื่อมีข้อมูลรีพอร์ตใหม่หลั่งไหลมาจาก AirView Agent โค้ดจะเขียนข้อมูลลงในฐานข้อมูล **ทั้ง 2 ที่พร้อมกัน**: 
    *   **Write 1:** บันทึกลง Eir (ฐานข้อมูลแพทย์ใหม่)
    *   **Write 2:** บันทึกลง Mega Care (ฐานข้อมูลเดิม เพื่อให้ Dashboard ไม่พัง)
3.  เปิดการประเมินรันโค้ดขั้นตอนนี้ไปสัก 1-2 สัปดาห์ เพื่อให้มั่นใจว่า Eir รับข้อมูลประจำวันได้ถูกต้อง

### Phase 2.3: The "Backfill" (ดูดข้อมูลเก่า)
1.  รัน Python Migration Script แบบ Batch Process (เช่น ตอนตี 2)
2.  อ่านข้อมูล `prescriptions` และ `daily_reports` **ย้อนหลังทั้งหมด** จาก Firestore/BigQuery ของ Mega Care
3.  Transform โครงสร้าง JSON ของ AirView ให้เข้ากับโครงสร้าง EMR ของ Eir
4.  บันทึกลง Eir (โดยข้ามข้อมูลช่วงสัปดาห์ปัจจุบันที่ใช้วิธี Dual-Write มาแล้ว ป้องกันข้อมูลซ้ำซ้อน)

### Phase 2.4: The "Cut-over" (สับสวิตช์ & ถอดท่อเก่า)
1.  อัปเดตแอปพลิเคชัน Frontend ของ Mega Care ให้ดึงข้อมูลจาก Eir API แทนการต่อตรงเข้า Firestore ตัวเอง (ในส่วนของการตั้งค่าเครื่อง และกราฟ AHI ระดับลึก)
2.  หยุดการเขียน (Write) ข้อมูล Clinical ลงใน Firestore ของ Mega Care 
3.  (ระยะยาว) ลบข้อมูล Clinical Data ออกจาก Mega Care Admin Portal (Soft Delete/Archive) 

---

## 🛠 3. แผนการอุดช่องโหว่ (Gap Analysis Implementation)
ทำขนานไปพร้อมกับการออกแบบ Database Schema

### 3.1 Telemed & Google Meet Security
*   **Google Workspace for Healthcare:** ให้ผู้ดูแลระบบไอที (IT Admin) ขอทำข้อตกลง Business Associate Agreement (BAA) กับ Google สำหรับโดเมนของโรงพยาบาล
*   **Recording Automation:**
    *   ใช้ Google Meet API 
    *   หลังคุยเสร็จ ให้ดึงลิงก์ไฟล์ MP4 จาก Google Drive ของหมอ แนบเข้าไปเป็น Metadata ในตารางนัดหมายของ Eir อัตโนมัติ พร้อมตั้งสิทธิ์การดู (Share Permission) ล็อกไว้ให้เฉพาะหมอเจ้าของไข้และ Eir Service Account

### 3.2 Secure Handoff (JWT & Magic Links)
*   **การเชื่อมต่อระหว่าง Portal:**
    *   แทนที่การส่ง User ID เฉยๆ ด้วยคำสั่ง **JSON Web Token (JWT) แบบมีวันหมดอายุ (Expiry 5-10 นาที)**
    *   ตัวอย่าง: `https://eir.clinic.com/login?token=eyJhb...` ระบบ Eir จะ Decode Token เช็กว่า Admin จาก Mega Care มีสิทธิ์เข้าดูหน้าจอสรุปเคสคนไข้รหัส `1001` จริงหรือไม่ (ไม่ต้องให้ Admin ล็อกอิน Eir ใหม่)

### 3.3 Clinical Workflow
*   **Alert & Escalation Button:** 
    *   หน้า Triage Dashboard ของ Mega Care (สำหรับเคสสีแดง) ต้องเพิ่มปุ่มป๊อปอัป "Escalate To Doctor" 
    *   ปุ่มนี้จะยิง Webhook ไปตีตั๋ว Urgent Queue แจ้งเตือนในกล่องข้อความบนจอ Eir ของพยาบาลวิชาชีพ หรือ Line แจ้งเตือนคุณหมอทันที

### 3.4 e-Consent
*   หน้าเว็บพอร์ทัลฝั่งคนไข้ ต้องมีกล่องติ๊กว่า *"ข้าพเจ้ายินยอมให้ข้อมูลทางแพทย์ (Sleep Test, CPAP Data) ถูกส่งต่อและจัดเก็บบนระบบ Eir ตามนโยบาย PDPA"* ติ๊กเสร็จให้ล็อกเวลาประทับตรา (Timestamp) เข้าฐานข้อมูล

---

## 📝 4. แผนฉุกเฉิน / Rollback Strategy
ถ้าสับสวิตช์ระบบแล้วพยาบาลหรือหมอแจ้งว่า "ระบบล่ม/ข้อมูลหาย" (Disaster Event)
1.  **Stop Migration Sync:** หยุดการซิงค์ข้อมูลใหม่ไป Eir ทันที
2.  **Revert to Mega Care:** สับสวิตช์ Frontend ให้อ่านข้อมูลจาก Mega Care ดั่งเดิม
3.  **Investigate:** ตรวจสอบ Error Log ค้นหา PII data leakage และแก้บัค
4.  **Restore:** หากข้อมูลทางแพทย์ใน Eir ปนเปื้อน ให้ Restore Database Eir จากแบ็กอัพ 1 ชั่วโมงล่าสุด (RPO < 1H)
5.  **Restart Backfill:** เริ่มกวาดข้อมูลใหม่เฉพาะส่วนที่หล่นหายไป

---
**Prepared By:** AI Systems Engineering Team

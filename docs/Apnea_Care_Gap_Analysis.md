# Telemedicine Compliance & Gap Analysis สำหรับ Sleep Apnea Care Flow

เอกสารฉบับนี้จัดทำขึ้นเพื่อวิเคราะห์ช่องว่าง (Gap Analysis) และเสนอแนะแนวทางการยกระดับกระบวนการทำงานแบบบูรณาการระหว่าง **Google Meet**, **Mega Care Admin Portal** และ **Eir** ให้เป็นไปตามมาตรฐานการแพทย์ทางไกล, กฎหมายคุ้มครองข้อมูลส่วนบุคคล (PDPA/HIPAA), และสามารถปฏิบัติงานได้จริงอย่างมีประสิทธิภาพสูงสุด

---

## 1. การปฏิบัติตามกฎหมายและเสถียรภาพของข้อมูล (PDPA & HIPAA Compliance)

การออกแบบให้ **Mega Care Admin Portal** จัดการ Data Operation และ **Eir** จัดการ Clinical Data เป็นแนวทาง Decoupled Architecture ที่ดี แต่ยังพบช่องว่างในการจัดการข้อมูลที่ต้องระมัดระวัง

### 1.1 ความปลอดภัยของแพลตฟอร์มวิดีโอ (Video Communication Security)
*   **Gap:** การใช้ Google Meet บัญชีองค์กรทั่วไป (หรือบัญชีฟรี) สำหรับ Telemedicine อาจไม่มีสัญญาปกปิดข้อมูล (BAA - Business Associate Agreement) ทำให้เสี่ยงต่อการถูกดักฟังหรือการนำข้อมูลไปใช้ทางการตลาด (Mining) ซึ่งผิดหลัก HIPAA/PDPA
*   **Recommendation:**
    *   ควรอัปเกรดและใช้ **Google Workspace for Healthcare (Enterprise)** ควบคู่กับการทำข้อตกลง BAA
    *   ตั้งค่า Policy ไม่บันทึก Video Recording ลง Cloud โดยพลการ เว้นแต่ได้รับ Patient Consent ก่อนเริ่มการสนทนาทุกครั้ง

### 1.2 สิทธิ์การเข้าถึงระดับตื้น-ลึก (Role-Based Access Control - RBAC)
*   **Gap:** แอดมินและทีมฝ่ายสนับสนุน (Operation/RT) ไม่ควรเห็นประวัติการตรวจวินิจฉัยโรคอื่นๆ (Diagnostic Notes / SOAP) ของคนไข้ในระบบ Eir เพื่อรักษาความลับผู้ป่วย (Doctor-Patient Privilege)
*   **Recommendation:**
    *   **Admin Portal:** กำหนดสิทธิ์ให้แอดมินหรือ RT เห็นตัวเลขสรุปเชิงเทคนิคเท่านั้น (AHI, Usage Hours, Mask Leak) และสถานะการชำระเงิน
    *   **Eir:** จำกัดการเข้าถึงบันทึกการรักษา (Clinical Progress Note) ให้เฉพาะบุคลากรการแพทย์ (แพทย์และพยาบาลผู้รับผิดชอบเคส) 

### 1.3 การบันทึกประวัติการถูกเข้าถึงข้อมูล (Audit Trailing)
*   **Gap:** หากมีพนักงานแกล้ง/แอบเปิดดูข้อมูลคนไข้ ระบบปัจจุบันอาจจะตามรอยไม่ได้
*   **Recommendation:**
    *   ปรับปรุง API และการล็อกอินเข้าสู่ **Eir** และ **Admin Portal** ให้มีการบันทึก Audit Log ชัดเจน (`Who, What, When`) สำหรับทุกๆ Event การเปิดส่อง (View) แฟ้มคนไข้

---

## 2. มาตรฐานการประกอบวิชาชีพทางโทรเวชกรรม (Clinical Telemedicine Standards)

มาตรฐานแพทยสภาว่าด้วยการวินิจฉัยและรักษาผ่านระบบโทรเวชกรรม (Telemedicine) มีข้อกำหนดเพิ่มเติมในด้านตัวบุคคลและแผนฉุกเฉิน

### 2.1 การยืนยันตัวบุคคลผู้รับการรักษา (Patient Identity Verification / KYC)
*   **Gap:** ผู้ที่เข้าคอล Google Meet อาจจงใจสวมสิทธิเป็นคนไข้เพื่อขอยา (Fraudulent Prescription Request)
*   **Recommendation:**
    *   ก่อนเริ่มการตรวจ แอดมิน/พยาบาล ต้องทำการ **Visual Check** ผ่านกล้อง (ให้เปรียบเทียบหน้าคนไข้กับรูปถ่ายในบัตร ปชช. ที่แนบไว้ตอนลงทะเบียน)
    *   ในระบบ **Eir** ควรมี Checkbox เล็กๆ บังคับให้แพทย์ติ๊กว่า `"Verified Patient Identity"` ก่อนที่ปุ่ม "Generate e-Prescription" จะสามารถกดได้

### 2.2 การให้ความยินยอมรับการรักษาแบบ Telemedicine (Informed Consent)
*   **Gap:** ผู้ป่วยอาจไม่เข้าใจข้อจำกัดของการตรวจผ่านกล้องเมื่อเทียบกับการมา รพ.
*   **Recommendation:**
    *   ระหว่างที่ผู้ป่วยนัดหมายใน **Mega Care Connect (Frontend)** ต้องมี Checkbox แบบบังคับติ๊ก (e-Consent) ยอมรับเงื่อนไขและข้อจำกัดของ Telemedicine และระบุชัดเจนว่าหากพบความซับซ้อน แพทย์สามารถปฏิเสธและส่งตัวเข้าโรงพยาบาลได้

### 2.3 แผนสำรองทางเครือข่าย (Network Fallback Protocol)
*   **Gap:** ปัญหาสายหลุดหรือคุณภาพภาพและเสียงไม่ดีพอจะทำให้การวินิจฉัยคลาดเคลื่อนได้
*   **Recommendation:**
    *   หากภาพล้มเหลว ระบบต้องมีโปรโตคอลให้แพทย์สลับไปหาโดยใช้ "เสียงทางโทรศัพท์ (Voice Call)"
    *   แพทย์ต้องสามารถลงบันทึกใน **Eir** ได้ว่า "เซสชันถูกเปลี่ยนเป็นการสนทนาทางเสียงเนื่องจากอุปสรรคทางโครงข่าย"

---

## 3. การเชื่อมโยงระดับปฏิบัติการ (Practical Operations & UX Integration)

การทำงานที่ง่ายและลื่นไหล จะช่วยลดภาระแพทย์-แอดมิน (Burnout) และทำงานได้โดยไร้รอยต่อ

### 3.1 การสลับแพลตฟอร์มอย่างปลอดภัย (Secure Contextual Handoff)
*   **Gap:** การวางลิงก์จาก `Admin Portal` เล็งทะลุไปหา `Eir` หากทำไม่ดี (ใช้ Plain Text ID) อาจเสี่ยงต่อการโจมตีแบบ **IDOR** (Insecure Direct Object Reference)
*   **Recommendation:**
    *   ใช้ Tokenization แบบ **Short-lived JWT** หรือ **One-time Magic Link** ในการกระโดดข้ามจาก Portal มา Eir แทนการใช้ URL แบบ `?patient_id=1234` แบบตรงๆ

### 3.2 ระบบการส่งต่อคิวฉุกเฉิน (Triage Escalation)
*   **Gap:** หาก Agent ของ **Mega Care Admin Portal** ตรวจพบเคสที่ AHI เกิน 50 (Critical Severe) แต่คิวนัดของแพทย์ครั้งถัดไปยังอยู่อีก 14 วัน People operation ปกติอาจละเลยเคสนี้ได้
*   **Recommendation:**
    *   สร้างปุ่มหรือฟังก์ชัน **"Escalate to Medical Team"** ใน Admin Portal ทันทีที่ Triage ขึ้นสีแดง (ฝั่ง Dashboard ของแอดมิน)
    *   ฟังก์ชันนี้จะดัน Notification เด้งเข้า **Eir** ในช่อง "Emergency / Urgent Review" เพื่อให้แพทย์สามารถเปิดพิจารณาแทรกคิวได้ทันที

### 3.3 การบันทึกวิดีโอหลักฐาน (Video Post-Processing)
*   **Gap:** แม้หมอรักษาเสร็จ ไฟล์บันทึกวิดีโอลงใน Google Drive แอดมินต้องเสียเวลาไปก๊อปลิงก์มาแปะเองในประวัติ Eir หรืออาจจะหาไฟล์ไม่เจอ
*   **Recommendation:**
    *   ใช้งาน Google Workspace API เพื่อให้ระบบทำการ Pull วิดีโอ Link กลับมาแปะในช่อง `Encounter History` บนหน้า **Eir** ของเซสชันการโทรในวันนั้นๆ โดยอัติโนมัติ

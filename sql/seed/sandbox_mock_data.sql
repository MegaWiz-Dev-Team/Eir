-- ============================================================
-- Seed Data: Sandbox Mock Patients
-- Purpose: 5 mock patients for testing consent gates + migration
-- Environment: openemr_sandbox ONLY
-- ============================================================

-- Patient 1: 🟢 Green (Active consent, good therapy data)
INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES (
    1001, 'สมชาย', 'ใจดี', '1975-06-15', 'Male', 'active',
    '123 ถ.สุขุมวิท', 'กรุงเทพฯ', 'กรุงเทพฯ', '10110', '081-234-5678'
);

-- Patient 2: 🟡 Yellow (Active consent, needs monitoring)
INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES (
    1002, 'สมหญิง', 'รักษ์สุข', '1980-03-22', 'Female', 'active',
    '456 ถ.พหลโยธิน', 'กรุงเทพฯ', 'กรุงเทพฯ', '10400', '089-876-5432'
);

-- Patient 3: 🔴 Red (Active consent, critical AHI)
INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES (
    1003, 'ประยุทธ์', 'แข็งแรง', '1968-11-08', 'Male', 'active',
    '789 ถ.รัชดาภิเษก', 'กรุงเทพฯ', 'กรุงเทพฯ', '10310', '062-111-2222'
);

-- Patient 4: ❌ Declined consent
INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES (
    1004, 'วิชัย', 'ไม่ยินยอม', '1990-01-30', 'Male', 'active',
    '321 ถ.ลาดพร้าว', 'กรุงเทพฯ', 'กรุงเทพฯ', '10230', '095-333-4444'
);

-- Patient 5: ⏳ Pending consent
INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES (
    1005, 'นภา', 'รอตอบ', '1985-07-12', 'Female', 'active',
    '654 ถ.เพชรบุรี', 'กรุงเทพฯ', 'กรุงเทพฯ', '10400', '088-555-6666'
);

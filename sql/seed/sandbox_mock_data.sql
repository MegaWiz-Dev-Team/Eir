-- ============================================================
-- Seed Data: Eir Sandbox Complete Mockup
-- Purpose: Full test data for CPAP Sync + LBF forms + migration_log
-- Environment: openemr_sandbox ONLY
-- Run AFTER: 001_migration_log.sql, 002_lbf_cpap_prescription.sql,
--            003_lbf_sleep_report.sql
-- ============================================================

-- ============================
-- 1. PATIENTS (5 mock patients)
-- ============================

INSERT INTO `patient_data` (
    `pid`, `fname`, `lname`, `DOB`, `sex`, `status`,
    `street`, `city`, `state`, `postal_code`, `phone_home`
) VALUES
    -- 🟢 Green — good therapy, active consent
    (1001, 'สมชาย', 'ใจดี', '1975-06-15', 'Male', 'active',
     '123 ถ.สุขุมวิท', 'กรุงเทพฯ', 'กรุงเทพฯ', '10110', '081-234-5678'),
    -- 🟡 Yellow — needs monitoring
    (1002, 'สมหญิง', 'รักษ์สุข', '1980-03-22', 'Female', 'active',
     '456 ถ.พหลโยธิน', 'กรุงเทพฯ', 'กรุงเทพฯ', '10400', '089-876-5432'),
    -- 🔴 Red — critical AHI
    (1003, 'ประยุทธ์', 'แข็งแรง', '1968-11-08', 'Male', 'active',
     '789 ถ.รัชดาภิเษก', 'กรุงเทพฯ', 'กรุงเทพฯ', '10310', '062-111-2222'),
    -- ❌ Consent declined
    (1004, 'วิชัย', 'ไม่ยินยอม', '1990-01-30', 'Male', 'active',
     '321 ถ.ลาดพร้าว', 'กรุงเทพฯ', 'กรุงเทพฯ', '10230', '095-333-4444'),
    -- ⏳ Consent pending
    (1005, 'นภา', 'รอตอบ', '1985-07-12', 'Female', 'active',
     '654 ถ.เพชรบุรี', 'กรุงเทพฯ', 'กรุงเทพฯ', '10400', '088-555-6666')
ON DUPLICATE KEY UPDATE `fname` = VALUES(`fname`);

-- ============================
-- 2. ENCOUNTERS (1 per patient for CPAP visits)
-- ============================

INSERT INTO `form_encounter` (
    `id`, `pid`, `encounter`, `date`, `reason`, `facility`, `facility_id`,
    `provider_id`, `billing_facility`, `sensitivity`, `pc_catid`
) VALUES
    (5001, 1001, 5001, '2026-03-01 09:00:00', 'CPAP Follow-up', 'Sandbox Clinic', 1, 1, 1, 'normal', 5),
    (5002, 1002, 5002, '2026-03-05 10:30:00', 'CPAP Follow-up', 'Sandbox Clinic', 1, 1, 1, 'normal', 5),
    (5003, 1003, 5003, '2026-03-10 14:00:00', 'CPAP Urgent Review', 'Sandbox Clinic', 1, 1, 1, 'normal', 5),
    (5004, 1004, 5004, '2026-03-12 11:00:00', 'CPAP Initial', 'Sandbox Clinic', 1, 1, 1, 'normal', 5),
    (5005, 1005, 5005, '2026-03-15 16:00:00', 'CPAP Initial', 'Sandbox Clinic', 1, 1, 1, 'normal', 5)
ON DUPLICATE KEY UPDATE `reason` = VALUES(`reason`);

-- Register forms for encounters
INSERT INTO `forms` (`id`, `encounter`, `form_id`, `form_name`, `pid`, `formdir`, `date`)
VALUES
    (6001, 5001, 5001, 'CPAP Prescription', 1001, 'LBFcpap', '2026-03-01'),
    (6002, 5002, 5002, 'CPAP Prescription', 1002, 'LBFcpap', '2026-03-05'),
    (6003, 5003, 5003, 'CPAP Prescription', 1003, 'LBFcpap', '2026-03-10'),
    (6004, 5001, 5011, 'Sleep Report Data', 1001, 'LBFsleep', '2026-03-20'),
    (6005, 5001, 5012, 'Sleep Report Data', 1001, 'LBFsleep', '2026-03-21'),
    (6006, 5002, 5013, 'Sleep Report Data', 1002, 'LBFsleep', '2026-03-20'),
    (6007, 5003, 5014, 'Sleep Report Data', 1003, 'LBFsleep', '2026-03-20')
ON DUPLICATE KEY UPDATE `form_name` = VALUES(`form_name`);

-- ============================
-- 3. LBF: CPAP PRESCRIPTION DATA
-- ============================
-- OpenEMR stores LBF data in `lbf_data` table:
--   form_id = encounter ID, field_id = LBF field name, field_value = value

-- Patient 1001 — ResMed AirSense 11 APAP
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5001, 'cpap_model',       'ResMed AirSense 11 AutoSet'),
    (5001, 'cpap_serial_no',   'SN-23490001'),
    (5001, 'cpap_manufacturer','ResMed'),
    (5001, 'therapy_mode',     'apap'),
    (5001, 'pressure_min',     '6'),
    (5001, 'pressure_max',     '16'),
    (5001, 'epr_level',        '3'),
    (5001, 'ramp_time',        '20'),
    (5001, 'mask_type',        'nasal'),
    (5001, 'mask_model',       'AirFit N30i'),
    (5001, 'mask_size',        'm'),
    (5001, 'humidifier_level', '5')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- Patient 1002 — Philips DreamStation 2
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5002, 'cpap_model',       'Philips DreamStation 2 Auto'),
    (5002, 'cpap_serial_no',   'SN-DS2-78001'),
    (5002, 'cpap_manufacturer','Philips Respironics'),
    (5002, 'therapy_mode',     'apap'),
    (5002, 'pressure_min',     '5'),
    (5002, 'pressure_max',     '15'),
    (5002, 'epr_level',        '2'),
    (5002, 'ramp_time',        '15'),
    (5002, 'mask_type',        'full_face'),
    (5002, 'mask_model',       'DreamWear Full Face'),
    (5002, 'mask_size',        's'),
    (5002, 'humidifier_level', '4')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- Patient 1003 — ResMed AirSense 10 (older device, critical patient)
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5003, 'cpap_model',       'ResMed AirSense 10 AutoSet'),
    (5003, 'cpap_serial_no',   'SN-AS10-55023'),
    (5003, 'cpap_manufacturer','ResMed'),
    (5003, 'therapy_mode',     'bipap'),
    (5003, 'pressure_min',     '10'),
    (5003, 'pressure_max',     '20'),
    (5003, 'epr_level',        '1'),
    (5003, 'ramp_time',        '5'),
    (5003, 'mask_type',        'full_face'),
    (5003, 'mask_model',       'AirFit F30i'),
    (5003, 'mask_size',        'l'),
    (5003, 'humidifier_level', '6')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- ============================
-- 4. LBF: SLEEP REPORT DATA
-- ============================

-- Patient 1001 — Day 1: Good data (Green)
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5011, 'report_date',       '2026-03-20'),
    (5011, 'report_type',       'detail'),
    (5011, 'usage_hours',       '7.5'),
    (5011, 'usage_seconds',     '27000'),
    (5011, 'total_usage_days',  '28'),
    (5011, 'ahi',               '2.3'),
    (5011, 'leak_median',       '4.2'),
    (5011, 'leak_95th',         '12.1'),
    (5011, 'pressure_median',   '9.5'),
    (5011, 'pressure_95th',     '12.8'),
    (5011, 'csr_duration',      '0'),
    (5011, 'triage_status',     'green'),
    (5011, 'pressure_efficacy', '94.2'),
    (5011, 'compliance_status', 'eligible'),
    (5011, 'clinical_notes',    'ผู้ป่วยใช้เครื่องสม่ำเสมอ ค่า AHI อยู่ในเกณฑ์ดี ไม่มีปัญหา leak'),
    (5011, 'source_gcs_uri',    'gs://mega-care-reports/detail-reports/SB-PT-001/2026/03/20/report.pdf')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- Patient 1001 — Day 2: Still good (Green)
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5012, 'report_date',       '2026-03-21'),
    (5012, 'report_type',       'detail'),
    (5012, 'usage_hours',       '6.8'),
    (5012, 'usage_seconds',     '24480'),
    (5012, 'total_usage_days',  '29'),
    (5012, 'ahi',               '1.9'),
    (5012, 'leak_median',       '3.8'),
    (5012, 'leak_95th',         '10.5'),
    (5012, 'pressure_median',   '9.2'),
    (5012, 'pressure_95th',     '12.1'),
    (5012, 'csr_duration',      '0'),
    (5012, 'triage_status',     'green'),
    (5012, 'pressure_efficacy', '95.1'),
    (5012, 'compliance_status', 'eligible'),
    (5012, 'clinical_notes',    'ข้อมูลต่อเนื่อง ค่าปกติทั้งหมด'),
    (5012, 'source_gcs_uri',    'gs://mega-care-reports/detail-reports/SB-PT-001/2026/03/21/report.pdf')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- Patient 1002 — Yellow: usage low, AHI moderate
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5013, 'report_date',       '2026-03-20'),
    (5013, 'report_type',       'detail'),
    (5013, 'usage_hours',       '3.2'),
    (5013, 'usage_seconds',     '11520'),
    (5013, 'total_usage_days',  '15'),
    (5013, 'ahi',               '8.7'),
    (5013, 'leak_median',       '18.5'),
    (5013, 'leak_95th',         '32.4'),
    (5013, 'pressure_median',   '11.2'),
    (5013, 'pressure_95th',     '14.5'),
    (5013, 'csr_duration',      '5'),
    (5013, 'triage_status',     'yellow'),
    (5013, 'pressure_efficacy', '72.8'),
    (5013, 'compliance_status', 'not_eligible'),
    (5013, 'clinical_notes',    'ผู้ป่วยใช้เครื่องน้อยกว่า 4 ชม. leak สูง อาจต้องปรับ mask'),
    (5013, 'source_gcs_uri',    'gs://mega-care-reports/detail-reports/SB-PT-002/2026/03/20/report.pdf')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- Patient 1003 — Red: very high AHI, critical
INSERT INTO `lbf_data` (`form_id`, `field_id`, `field_value`) VALUES
    (5014, 'report_date',       '2026-03-20'),
    (5014, 'report_type',       'detail'),
    (5014, 'usage_hours',       '5.1'),
    (5014, 'usage_seconds',     '18360'),
    (5014, 'total_usage_days',  '22'),
    (5014, 'ahi',               '28.4'),
    (5014, 'leak_median',       '25.3'),
    (5014, 'leak_95th',         '45.8'),
    (5014, 'pressure_median',   '15.2'),
    (5014, 'pressure_95th',     '19.8'),
    (5014, 'csr_duration',      '35'),
    (5014, 'triage_status',     'red'),
    (5014, 'pressure_efficacy', '55.1'),
    (5014, 'compliance_status', 'not_eligible'),
    (5014, 'clinical_notes',    'AHI สูงมาก 28.4 ต้องพบแพทย์ด่วน Leak สูง อาจต้องเปลี่ยน mask CSR duration 35 นาที ส่งต่อ sleep specialist'),
    (5014, 'source_gcs_uri',    'gs://mega-care-reports/detail-reports/SB-PT-003/2026/03/20/report.pdf')
ON DUPLICATE KEY UPDATE `field_value` = VALUES(`field_value`);

-- ============================
-- 5. MIGRATION LOG (sample entries)
-- ============================

INSERT INTO `migration_log` (
    `patient_id`, `openemr_pid`, `data_type`, `source_doc_id`,
    `status`, `error_message`, `idempotency_key`
) VALUES
    -- Successful prescriptions
    ('SB-PT-001', 1001, 'prescription', 'CPAP-RX-2026-001', 'success', NULL,
     'SB-PT-001:prescription:CPAP-RX-2026-001'),
    ('SB-PT-002', 1002, 'prescription', 'CPAP-RX-2026-002', 'success', NULL,
     'SB-PT-002:prescription:CPAP-RX-2026-002'),
    ('SB-PT-003', 1003, 'prescription', 'CPAP-RX-2026-003', 'success', NULL,
     'SB-PT-003:prescription:CPAP-RX-2026-003'),

    -- Successful daily reports
    ('SB-PT-001', 1001, 'daily_report', '2026-03-20', 'success', NULL,
     'SB-PT-001:daily_report:2026-03-20'),
    ('SB-PT-001', 1001, 'daily_report', '2026-03-21', 'success', NULL,
     'SB-PT-001:daily_report:2026-03-21'),
    ('SB-PT-002', 1002, 'daily_report', '2026-03-20', 'success', NULL,
     'SB-PT-002:daily_report:2026-03-20'),
    ('SB-PT-003', 1003, 'daily_report', '2026-03-20', 'success', NULL,
     'SB-PT-003:daily_report:2026-03-20'),

    -- Skipped (declined consent)
    ('SB-PT-004', 1004, 'daily_report', '2026-03-20', 'skipped',
     'Patient consent status: declined',
     'SB-PT-004:daily_report:2026-03-20'),

    -- Pending (awaiting consent)
    ('SB-PT-005', 1005, 'daily_report', '2026-03-20', 'pending',
     'Patient consent status: pending',
     'SB-PT-005:daily_report:2026-03-20'),

    -- Failed example
    ('SB-PT-003', 1003, 'compliance_report', '2026-Q1', 'failed',
     'GCS file not found: gs://mega-care-reports/compliance/SB-PT-003/2026-Q1.pdf',
     'SB-PT-003:compliance_report:2026-Q1')
ON DUPLICATE KEY UPDATE `status` = VALUES(`status`);

-- ============================
-- 6. VERIFY COUNTS
-- ============================
-- Expected results after seed:
--   patient_data:   5 rows (pid 1001-1005)
--   form_encounter: 5 rows (encounter 5001-5005)
--   forms:          7 rows (3 CPAP + 4 Sleep)
--   lbf_data:       12*3 + 16*4 = 100 rows
--   migration_log:  10 rows

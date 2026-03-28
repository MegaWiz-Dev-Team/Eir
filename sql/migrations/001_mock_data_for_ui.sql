-- 001_mock_data_for_ui.sql
-- Missing OpenEMR tables and mock data for UI buttons

-- Clear previous mock data to prevent duplicates
DELETE FROM `lists` WHERE `pid` IN (1, 2);
DELETE FROM `prescriptions` WHERE `patient_id` IN (1, 2);
DELETE FROM `openemr_postcalendar_events` WHERE `pc_pid` IN (1, 2);

-- Mock Data for Somchai (pid: 1)
-- Medical Problem: Obstructive Sleep Apnea
INSERT INTO `lists` (`type`, `title`, `diagnosis`, `pid`, `date`, `activity`, `subtype`, `reaction`, `verification`, `outcome`) VALUES 
('medical_problem', 'Obstructive Sleep Apnea (OSA)', 'G47.33', 1, '2023-05-10 10:00:00', 1, '', '', '', 0),
('medical_problem', 'Hypertension', 'I10', 1, '2022-01-15 09:30:00', 1, '', '', '', 0),
('allergy', 'Penicillin', 'Allergy to Penicillin', 1, '2020-08-20 14:00:00', 1, '', 'Hives', '', 0);

-- Medications
-- prescriptions required: txDate, usage_category_title, request_intent_title
INSERT INTO `prescriptions` (`patient_id`, `drug`, `dosage`, `quantity`, `route`, `interval`, `date_added`, `start_date`, `active`, `txDate`, `usage_category_title`, `request_intent_title`) VALUES 
(1, 'Losartan', '50mg', '30', 'Oral', NULL, '2023-01-15 10:00:00', '2023-01-15', 1, '2023-01-15', '', ''),
(1, 'CPAP Therapy (ResMed AirSense 10)', '10 cmH2O', '1', 'Inhalation', NULL, '2023-05-15 11:00:00', '2023-05-15', 1, '2023-05-15', '', '');

-- Appointments
-- openemr_postcalendar_events required: pc_multiple(0), pc_topic(1), pc_recurrtype(0), pc_recurrfreq(0), pc_room('')
INSERT INTO `openemr_postcalendar_events` (`pc_pid`, `pc_title`, `pc_catid`, `pc_time`, `pc_eventDate`, `pc_endDate`, `pc_duration`, `pc_apptstatus`, `pc_multiple`, `pc_topic`, `pc_recurrtype`, `pc_recurrfreq`, `pc_room`) VALUES 
(1, 'CPAP Follow-up & Sleep Study analysis', 5, '2026-04-10 10:00:00', '2026-04-10', '2026-04-10', 30, '-', 0, 1, 0, 0, ''),
(1, 'Annual Checkup', 5, '2026-05-20 13:00:00', '2026-05-20', '2026-05-20', 45, '-', 0, 1, 0, 0, '');

-- Mock Data for Somsri (pid: 2)
INSERT INTO `openemr_postcalendar_events` (`pc_pid`, `pc_title`, `pc_catid`, `pc_time`, `pc_eventDate`, `pc_endDate`, `pc_duration`, `pc_apptstatus`, `pc_multiple`, `pc_topic`, `pc_recurrtype`, `pc_recurrfreq`, `pc_room`) VALUES 
(2, 'Initial Sleep Assessment', 5, '2026-04-05 15:00:00', '2026-04-05', '2026-04-05', 30, '-', 0, 1, 0, 0, '');

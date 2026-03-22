-- ============================================================
-- Migration 003: LBF Form — Sleep Report Data
-- Purpose: Layout-Based Form for daily/compliance sleep therapy reports
-- Sprint: 5 (Data Migration Readiness)
-- ============================================================

-- Create the LBF group
INSERT INTO `layout_group_properties` (`grp_form_id`, `grp_group_id`, `grp_title`, `grp_subtitle`, `grp_mapping`, `grp_activity`)
VALUES
    ('LBFsleep', '', 'Sleep Report Data', '', '', 1),
    ('LBFsleep', '1', 'Usage Summary', '', '', 1),
    ('LBFsleep', '2', 'Therapy Results', '', '', 1),
    ('LBFsleep', '3', 'Clinical Assessment', '', '', 1)
ON DUPLICATE KEY UPDATE `grp_title` = VALUES(`grp_title`);

-- Usage Summary fields
INSERT INTO `layout_options` (
    `form_id`, `field_id`, `group_id`, `title`, `seq`, `data_type`,
    `uor`, `fld_length`, `max_length`, `list_id`, `titlecols`, `datacols`,
    `default_value`, `edit_options`, `description`, `fld_rows`
) VALUES
    ('LBFsleep', 'report_date',        '1', 'Report Date',         10, 4, 1, 10, 10, '', 1, 1, '', '', 'Date of the sleep report', 0),
    ('LBFsleep', 'report_type',        '1', 'Report Type',         15, 1, 1, 0,  0, 'LBFsleep_report_type', 1, 1, '', '', 'Detail or Compliance report', 0),
    ('LBFsleep', 'usage_hours',        '1', 'Usage (hours)',       20, 2, 1, 10, 10, '', 1, 1, '', '', 'Average daily usage in hours', 0),
    ('LBFsleep', 'usage_seconds',      '1', 'Usage (seconds)',     25, 2, 1, 10, 10, '', 1, 1, '', '', 'Usage time in seconds (raw)', 0),
    ('LBFsleep', 'total_usage_days',   '1', 'Total Days Used',     30, 2, 1, 10, 10, '', 1, 1, '', '', 'Number of days with usage ≥ 4 hours', 0),
    -- Therapy Results fields
    ('LBFsleep', 'ahi',                '2', 'AHI (events/hr)',     10, 2, 1, 10, 10, '', 1, 1, '', '', 'Apnea-Hypopnea Index', 0),
    ('LBFsleep', 'leak_median',        '2', 'Leak Median (L/min)', 20, 2, 1, 10, 10, '', 1, 1, '', '', 'Median unintentional leak rate', 0),
    ('LBFsleep', 'leak_95th',          '2', 'Leak 95th %ile',     30, 2, 1, 10, 10, '', 1, 1, '', '', '95th percentile leak rate', 0),
    ('LBFsleep', 'pressure_median',    '2', 'Pressure Median',    40, 2, 1, 10, 10, '', 1, 1, '', '', 'Median therapy pressure (cmH₂O)', 0),
    ('LBFsleep', 'pressure_95th',      '2', 'Pressure 95th %ile', 50, 2, 1, 10, 10, '', 1, 1, '', '', '95th percentile pressure', 0),
    ('LBFsleep', 'csr_duration',       '2', 'CSR Duration (min)',  60, 2, 1, 10, 10, '', 1, 1, '', '', 'Cheyne-Stokes Respiration duration', 0),
    -- Clinical Assessment fields
    ('LBFsleep', 'triage_status',      '3', 'Triage Status',      10, 1, 1, 0, 0, 'LBFsleep_triage', 1, 1, '', '', 'Green/Yellow/Red based on therapy data', 0),
    ('LBFsleep', 'pressure_efficacy',  '3', 'Pressure Efficacy (%)', 20, 2, 1, 10, 10, '', 1, 1, '', '', 'Calculated pressure efficacy percentage', 0),
    ('LBFsleep', 'compliance_status',  '3', 'Compliance Eligible', 30, 1, 1, 0, 0, 'LBFsleep_compliance', 1, 1, '', '', 'Insurance compliance eligibility', 0),
    ('LBFsleep', 'clinical_notes',     '3', 'Clinical Notes',     40, 3, 1, 40, 500, '', 1, 1, '', '', 'Additional clinical observations', 4),
    ('LBFsleep', 'source_gcs_uri',     '3', 'Source Report URI',  50, 2, 1, 60, 500, '', 1, 1, '', '', 'GCS URI of the original PDF report', 0)
ON DUPLICATE KEY UPDATE `title` = VALUES(`title`);

-- Create list options for dropdown fields
INSERT INTO `list_options` (`list_id`, `option_id`, `title`, `seq`, `is_default`, `activity`)
VALUES
    -- Report types
    ('LBFsleep_report_type', 'detail', 'Detail Report', 10, 1, 1),
    ('LBFsleep_report_type', 'compliance', 'Compliance Report', 20, 0, 1),
    -- Triage statuses
    ('LBFsleep_triage', 'green', '🟢 Green (Normal)', 10, 0, 1),
    ('LBFsleep_triage', 'yellow', '🟡 Yellow (Monitor)', 20, 0, 1),
    ('LBFsleep_triage', 'red', '🔴 Red (Action Required)', 30, 0, 1),
    -- Compliance eligibility
    ('LBFsleep_compliance', 'eligible', 'Eligible', 10, 0, 1),
    ('LBFsleep_compliance', 'not_eligible', 'Not Eligible', 20, 0, 1),
    ('LBFsleep_compliance', 'pending', 'Pending Review', 30, 1, 1)
ON DUPLICATE KEY UPDATE `title` = VALUES(`title`);

-- Register the LBF form
INSERT INTO `registry` (`name`, `state`, `directory`, `sql_run`, `category`)
VALUES ('Sleep Report Data', 1, 'LBFsleep', 1, 'Clinical')
ON DUPLICATE KEY UPDATE `name` = VALUES(`name`);

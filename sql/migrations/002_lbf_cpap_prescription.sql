-- ============================================================
-- Migration 002: LBF Form — CPAP Prescription
-- Purpose: Layout-Based Form for CPAP therapy prescription data
-- Sprint: 5 (Data Migration Readiness)
-- ============================================================

-- Create the LBF group
INSERT INTO `layout_group_properties` (`grp_form_id`, `grp_group_id`, `grp_title`, `grp_subtitle`, `grp_mapping`, `grp_activity`)
VALUES
    ('LBFcpap', '', 'CPAP Prescription', '', '', 1),
    ('LBFcpap', '1', 'Device Information', '', '', 1),
    ('LBFcpap', '2', 'Therapy Settings', '', '', 1),
    ('LBFcpap', '3', 'Mask & Accessories', '', '', 1)
ON DUPLICATE KEY UPDATE `grp_title` = VALUES(`grp_title`);

-- Device Information fields
INSERT INTO `layout_options` (
    `form_id`, `field_id`, `group_id`, `title`, `seq`, `data_type`,
    `uor`, `fld_length`, `max_length`, `list_id`, `titlecols`, `datacols`,
    `default_value`, `edit_options`, `description`, `fld_rows`
) VALUES
    ('LBFcpap', 'cpap_model',      '1', 'CPAP Model',       10, 2, 1, 30, 100, '', 1, 1, '', '', 'CPAP device model (e.g., ResMed AirSense 11)', 0),
    ('LBFcpap', 'cpap_serial_no',  '1', 'Serial Number',    20, 2, 1, 30, 100, '', 1, 1, '', '', 'Device serial number', 0),
    ('LBFcpap', 'cpap_manufacturer','1', 'Manufacturer',     30, 2, 1, 30, 100, '', 1, 1, '', '', 'Device manufacturer', 0),
    -- Therapy Settings fields
    ('LBFcpap', 'therapy_mode',    '2', 'Therapy Mode',     10, 1, 1, 0, 0, 'LBFcpap_therapy_mode', 1, 1, '', '', 'CPAP/APAP/BiPAP/ASV', 0),
    ('LBFcpap', 'pressure_min',    '2', 'Min Pressure (cmH₂O)', 20, 2, 1, 10, 10, '', 1, 1, '', '', 'Minimum pressure setting', 0),
    ('LBFcpap', 'pressure_max',    '2', 'Max Pressure (cmH₂O)', 30, 2, 1, 10, 10, '', 1, 1, '', '', 'Maximum pressure setting', 0),
    ('LBFcpap', 'epr_level',       '2', 'EPR Level',        40, 1, 1, 0, 0, 'LBFcpap_epr', 1, 1, '', '', 'Expiratory Pressure Relief level (0-3)', 0),
    ('LBFcpap', 'ramp_time',       '2', 'Ramp Time (min)',  50, 2, 1, 10, 10, '', 1, 1, '', '', 'Ramp-up time in minutes', 0),
    -- Mask & Accessories fields
    ('LBFcpap', 'mask_type',       '3', 'Mask Type',        10, 1, 1, 0, 0, 'LBFcpap_mask', 1, 1, '', '', 'Nasal/Full Face/Nasal Pillow', 0),
    ('LBFcpap', 'mask_model',      '3', 'Mask Model',       20, 2, 1, 30, 100, '', 1, 1, '', '', 'Specific mask model name', 0),
    ('LBFcpap', 'mask_size',       '3', 'Mask Size',        30, 1, 1, 0, 0, 'LBFcpap_mask_size', 1, 1, '', '', 'S/M/L/XL', 0),
    ('LBFcpap', 'humidifier_level','3', 'Humidifier Level', 40, 2, 1, 10, 10, '', 1, 1, '', '', 'Humidifier setting (0-8)', 0)
ON DUPLICATE KEY UPDATE `title` = VALUES(`title`);

-- Create list options for dropdown fields
INSERT INTO `list_options` (`list_id`, `option_id`, `title`, `seq`, `is_default`, `activity`)
VALUES
    -- Therapy modes
    ('LBFcpap_therapy_mode', 'cpap', 'CPAP (Fixed)', 10, 1, 1),
    ('LBFcpap_therapy_mode', 'apap', 'APAP (Auto)', 20, 0, 1),
    ('LBFcpap_therapy_mode', 'bipap', 'BiPAP', 30, 0, 1),
    ('LBFcpap_therapy_mode', 'asv', 'ASV', 40, 0, 1),
    -- EPR levels
    ('LBFcpap_epr', '0', 'Off', 10, 0, 1),
    ('LBFcpap_epr', '1', 'Level 1', 20, 0, 1),
    ('LBFcpap_epr', '2', 'Level 2', 30, 0, 1),
    ('LBFcpap_epr', '3', 'Level 3', 40, 1, 1),
    -- Mask types
    ('LBFcpap_mask', 'nasal', 'Nasal Mask', 10, 0, 1),
    ('LBFcpap_mask', 'full_face', 'Full Face Mask', 20, 1, 1),
    ('LBFcpap_mask', 'nasal_pillow', 'Nasal Pillow', 30, 0, 1),
    ('LBFcpap_mask', 'hybrid', 'Hybrid Mask', 40, 0, 1),
    -- Mask sizes
    ('LBFcpap_mask_size', 'xs', 'XS', 10, 0, 1),
    ('LBFcpap_mask_size', 's', 'S', 20, 0, 1),
    ('LBFcpap_mask_size', 'm', 'M', 30, 1, 1),
    ('LBFcpap_mask_size', 'l', 'L', 40, 0, 1),
    ('LBFcpap_mask_size', 'xl', 'XL', 50, 0, 1)
ON DUPLICATE KEY UPDATE `title` = VALUES(`title`);

-- Register the LBF form in the registry
INSERT INTO `registry` (`name`, `state`, `directory`, `sql_run`, `category`)
VALUES ('CPAP Prescription', 1, 'LBFcpap', 1, 'Clinical')
ON DUPLICATE KEY UPDATE `name` = VALUES(`name`);

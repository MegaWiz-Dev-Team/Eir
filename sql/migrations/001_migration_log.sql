-- ============================================================
-- Migration 001: Create migration_log table
-- Purpose: Track data migration from Mega Care → Eir (OpenEMR)
-- Sprint: 5 (PDPA Consent Remediation & Data Migration)
-- ============================================================

CREATE TABLE IF NOT EXISTS `migration_log` (
    `id`              INT AUTO_INCREMENT PRIMARY KEY,
    `patient_id`      VARCHAR(255) NOT NULL COMMENT 'Mega Care patient ID (Firestore doc ID)',
    `openemr_pid`     INT DEFAULT NULL COMMENT 'OpenEMR patient PID (after mapping)',
    `data_type`       ENUM(
                          'demographics',
                          'prescription',
                          'daily_report',
                          'compliance_report',
                          'appointment',
                          'consent'
                      ) NOT NULL COMMENT 'Type of data being migrated',
    `source_doc_id`   VARCHAR(255) DEFAULT NULL COMMENT 'Firestore document ID or GCS URI',
    `status`          ENUM('pending', 'success', 'failed', 'skipped') DEFAULT 'pending',
    `error_message`   TEXT DEFAULT NULL COMMENT 'Error details if migration failed',
    `idempotency_key` VARCHAR(512) DEFAULT NULL COMMENT 'Unique key to prevent duplicate migrations',
    `migrated_at`     TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    `updated_at`      TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY `unique_migration` (`patient_id`, `data_type`, `source_doc_id`),
    INDEX `idx_patient_status` (`patient_id`, `status`),
    INDEX `idx_idempotency` (`idempotency_key`),
    INDEX `idx_data_type_status` (`data_type`, `status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
  COMMENT='Tracks data migration from Mega Care Firestore to Eir OpenEMR';

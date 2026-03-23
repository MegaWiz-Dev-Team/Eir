-- ============================================================
-- Sandbox Bootstrap: Minimal OpenEMR schema for Eir sandbox
-- Purpose: Create ONLY the tables needed for CPAP/Sleep LBF testing
-- This is NOT a full OpenEMR install — just the tables for mockup
-- ============================================================

-- Patient demographics
CREATE TABLE IF NOT EXISTS `patient_data` (
    `id`            BIGINT NOT NULL AUTO_INCREMENT,
    `pid`           BIGINT NOT NULL DEFAULT 0,
    `fname`         VARCHAR(255) DEFAULT NULL,
    `lname`         VARCHAR(255) DEFAULT NULL,
    `DOB`           DATE DEFAULT NULL,
    `sex`           VARCHAR(50) DEFAULT NULL,
    `status`        VARCHAR(50) DEFAULT NULL,
    `street`        VARCHAR(255) DEFAULT NULL,
    `city`          VARCHAR(255) DEFAULT NULL,
    `state`         VARCHAR(255) DEFAULT NULL,
    `postal_code`   VARCHAR(255) DEFAULT NULL,
    `phone_home`    VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `pid` (`pid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Encounters
CREATE TABLE IF NOT EXISTS `form_encounter` (
    `id`                BIGINT NOT NULL AUTO_INCREMENT,
    `pid`               BIGINT DEFAULT NULL,
    `encounter`         BIGINT DEFAULT NULL,
    `date`              DATETIME DEFAULT NULL,
    `reason`            TEXT,
    `facility`          TEXT,
    `facility_id`       INT DEFAULT NULL,
    `provider_id`       INT DEFAULT NULL,
    `billing_facility`  INT DEFAULT NULL,
    `sensitivity`       VARCHAR(30) DEFAULT NULL,
    `pc_catid`          INT DEFAULT NULL,
    PRIMARY KEY (`id`),
    KEY `encounter` (`encounter`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Form registry (links forms to encounters)
CREATE TABLE IF NOT EXISTS `forms` (
    `id`            BIGINT NOT NULL AUTO_INCREMENT,
    `encounter`     BIGINT DEFAULT NULL,
    `form_id`       BIGINT DEFAULT NULL,
    `form_name`     VARCHAR(255) DEFAULT NULL,
    `pid`           BIGINT DEFAULT NULL,
    `formdir`       VARCHAR(255) DEFAULT NULL,
    `date`          DATE DEFAULT NULL,
    `deleted`       TINYINT DEFAULT 0,
    PRIMARY KEY (`id`),
    KEY `encounter` (`encounter`),
    KEY `pid` (`pid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- LBF data storage (key-value per form instance)
CREATE TABLE IF NOT EXISTS `lbf_data` (
    `form_id`       BIGINT NOT NULL,
    `field_id`      VARCHAR(31) NOT NULL,
    `field_value`   LONGTEXT,
    PRIMARY KEY (`form_id`, `field_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- LBF group properties (form sections)
CREATE TABLE IF NOT EXISTS `layout_group_properties` (
    `grp_form_id`   VARCHAR(31) NOT NULL DEFAULT '',
    `grp_group_id`  VARCHAR(31) NOT NULL DEFAULT '',
    `grp_title`     VARCHAR(63) NOT NULL DEFAULT '',
    `grp_subtitle`  VARCHAR(63) NOT NULL DEFAULT '',
    `grp_mapping`   VARCHAR(31) NOT NULL DEFAULT '',
    `grp_activity`  TINYINT NOT NULL DEFAULT 1,
    PRIMARY KEY (`grp_form_id`, `grp_group_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- LBF field definitions
CREATE TABLE IF NOT EXISTS `layout_options` (
    `form_id`       VARCHAR(31) NOT NULL DEFAULT '',
    `field_id`      VARCHAR(31) NOT NULL DEFAULT '',
    `group_id`      VARCHAR(31) NOT NULL DEFAULT '',
    `title`         VARCHAR(63) NOT NULL DEFAULT '',
    `seq`           INT NOT NULL DEFAULT 0,
    `data_type`     TINYINT NOT NULL DEFAULT 0,
    `uor`           TINYINT NOT NULL DEFAULT 1,
    `fld_length`    INT NOT NULL DEFAULT 15,
    `max_length`    INT NOT NULL DEFAULT 0,
    `list_id`       VARCHAR(100) NOT NULL DEFAULT '',
    `titlecols`     TINYINT NOT NULL DEFAULT 1,
    `datacols`      TINYINT NOT NULL DEFAULT 1,
    `default_value` VARCHAR(255) NOT NULL DEFAULT '',
    `edit_options`  VARCHAR(36) NOT NULL DEFAULT '',
    `description`   TEXT,
    `fld_rows`      INT NOT NULL DEFAULT 0,
    PRIMARY KEY (`form_id`, `field_id`, `seq`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- List options (dropdown values)
CREATE TABLE IF NOT EXISTS `list_options` (
    `list_id`       VARCHAR(100) NOT NULL DEFAULT '',
    `option_id`     VARCHAR(100) NOT NULL DEFAULT '',
    `title`         VARCHAR(255) NOT NULL DEFAULT '',
    `seq`           INT NOT NULL DEFAULT 0,
    `is_default`    TINYINT NOT NULL DEFAULT 0,
    `activity`      TINYINT NOT NULL DEFAULT 1,
    PRIMARY KEY (`list_id`, `option_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Form registry (installed forms)
CREATE TABLE IF NOT EXISTS `registry` (
    `name`          VARCHAR(255) DEFAULT NULL,
    `state`         TINYINT DEFAULT 1,
    `directory`     VARCHAR(255) DEFAULT NULL,
    `sql_run`       TINYINT DEFAULT 0,
    `category`      VARCHAR(255) DEFAULT NULL,
    UNIQUE KEY `directory` (`directory`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

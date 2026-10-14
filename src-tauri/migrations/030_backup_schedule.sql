-- ============================================================
-- MIGRACION 030: Programacion de backups
--   schedule_days: mascara de bits, lunes = 1 ... domingo = 64 (127 = todos).
--   schedule_time: hora local del PC.
-- ============================================================
ALTER TABLE backup_settings
    ADD COLUMN auto_enabled        BOOLEAN  NOT NULL DEFAULT TRUE,
    ADD COLUMN schedule_time       TIME     NOT NULL DEFAULT '21:00',
    ADD COLUMN schedule_days       SMALLINT NOT NULL DEFAULT 127,
    ADD COLUMN warn_after_days     INT      NOT NULL DEFAULT 2,
    ADD COLUMN critical_after_days INT      NOT NULL DEFAULT 4,
    ADD CONSTRAINT ck_backup_settings_days
        CHECK (schedule_days BETWEEN 1 AND 127),
    ADD CONSTRAINT ck_backup_settings_warn
        CHECK (warn_after_days BETWEEN 1 AND 30),
    ADD CONSTRAINT ck_backup_settings_critical
        CHECK (critical_after_days > warn_after_days AND critical_after_days <= 60);

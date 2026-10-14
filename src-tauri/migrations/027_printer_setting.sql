-- ============================================================
-- MIGRACIÓN 027: Nombre de impresora térmica en app_settings
-- ============================================================

ALTER TABLE app_settings
    ADD COLUMN IF NOT EXISTS printer_name VARCHAR(100) DEFAULT 'DIG58IIA';

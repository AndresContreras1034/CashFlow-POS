ALTER TABLE backup_settings
    ALTER COLUMN encrypt_enabled SET DEFAULT TRUE;

UPDATE backup_settings
SET encrypt_enabled = TRUE
WHERE id = 1;

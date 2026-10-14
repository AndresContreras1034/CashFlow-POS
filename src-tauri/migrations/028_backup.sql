-- ============================================================
-- MIGRACION 028: Backups (nucleo)
-- ============================================================
--   * backup_settings: fila unica (id = 1), igual que app_settings.
--   * backup_history: una fila por intento TERMINADO (exito o fallo).
--     No hay estado "en curso" para que el dump no se copie a si mismo.
--   * El historial no se borra. Cuando la rotacion elimina el archivo
--     se marca file_deleted_at.
-- ============================================================

CREATE TABLE backup_settings (
    id            SMALLINT     PRIMARY KEY DEFAULT 1,
    local_dir     TEXT,                         -- NULL = Documentos\CashFlow-Backups
    pg_bin_dir    TEXT,                         -- NULL = deteccion automatica
    keep_daily    INT          NOT NULL DEFAULT 7,
    keep_weekly   INT          NOT NULL DEFAULT 4,
    keep_monthly  INT          NOT NULL DEFAULT 6,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT ck_backup_settings_singleton CHECK (id = 1),
    CONSTRAINT ck_backup_settings_daily     CHECK (keep_daily   BETWEEN 1 AND 90),
    CONSTRAINT ck_backup_settings_weekly    CHECK (keep_weekly  BETWEEN 0 AND 52),
    CONSTRAINT ck_backup_settings_monthly   CHECK (keep_monthly BETWEEN 0 AND 60),
    CONSTRAINT ck_backup_settings_dir_blank
        CHECK (local_dir IS NULL OR char_length(btrim(local_dir)) > 0),
    CONSTRAINT ck_backup_settings_bin_blank
        CHECK (pg_bin_dir IS NULL OR char_length(btrim(pg_bin_dir)) > 0)
);

INSERT INTO backup_settings (id) VALUES (1);

CREATE TRIGGER trg_backup_settings_updated_at
    BEFORE UPDATE ON backup_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TABLE backup_history (
    id                 SERIAL PRIMARY KEY,
    kind               VARCHAR(20)  NOT NULL,
    status             VARCHAR(10)  NOT NULL,
    started_at         TIMESTAMPTZ  NOT NULL,
    finished_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    duration_ms        BIGINT       NOT NULL,

    file_name          VARCHAR(255),
    file_path          TEXT,
    size_bytes         BIGINT,
    sha256             VARCHAR(64),
    pg_server_version  VARCHAR(30),
    last_migration     BIGINT,

    error_message      TEXT,
    created_by         VARCHAR(100) NOT NULL DEFAULT 'system',
    file_deleted_at    TIMESTAMPTZ,
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT ck_backup_history_kind
        CHECK (kind IN ('manual', 'scheduled', 'catchup', 'pre_restore', 'pre_import')),
    CONSTRAINT ck_backup_history_status
        CHECK (status IN ('success', 'failed')),
    CONSTRAINT ck_backup_history_duration
        CHECK (duration_ms >= 0),
    CONSTRAINT ck_backup_history_outcome
        CHECK (
            (status = 'success'
                AND file_name IS NOT NULL AND file_path IS NOT NULL
                AND size_bytes IS NOT NULL AND sha256 IS NOT NULL)
            OR
            (status = 'failed' AND error_message IS NOT NULL)
        )
);

CREATE INDEX idx_backup_history_started ON backup_history (started_at DESC);
CREATE INDEX idx_backup_history_success ON backup_history (started_at DESC)
    WHERE status = 'success' AND file_deleted_at IS NULL;

CREATE OR REPLACE FUNCTION prevent_backup_history_deletion()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'backup_history es de solo lectura: % no esta permitido', TG_OP
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_backup_history_no_delete
    BEFORE DELETE ON backup_history
    FOR EACH ROW EXECUTE FUNCTION prevent_backup_history_deletion();
CREATE TRIGGER trg_backup_history_no_truncate
    BEFORE TRUNCATE ON backup_history
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_backup_history_deletion();

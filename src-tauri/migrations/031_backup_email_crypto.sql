-- ============================================================
-- MIGRACION 031: Cifrado, SMTP, destinatarios y envios
--   Las contrasenas NO se guardan aqui (van al almacen de Windows).
-- ============================================================
ALTER TABLE backup_settings
    ADD COLUMN encrypt_enabled  BOOLEAN      NOT NULL DEFAULT FALSE,
    ADD COLUMN smtp_enabled     BOOLEAN      NOT NULL DEFAULT FALSE,
    ADD COLUMN smtp_host        VARCHAR(255) NOT NULL DEFAULT 'smtp.gmail.com',
    ADD COLUMN smtp_port        INT          NOT NULL DEFAULT 465,
    ADD COLUMN smtp_security    VARCHAR(10)  NOT NULL DEFAULT 'tls',
    ADD COLUMN smtp_username    VARCHAR(255),
    ADD COLUMN smtp_from_name   VARCHAR(100),
    ADD COLUMN email_on_failure BOOLEAN      NOT NULL DEFAULT TRUE,
    ADD CONSTRAINT ck_backup_settings_smtp_port
        CHECK (smtp_port BETWEEN 1 AND 65535),
    ADD CONSTRAINT ck_backup_settings_smtp_security
        CHECK (smtp_security IN ('tls', 'starttls')),
    ADD CONSTRAINT ck_backup_settings_smtp_host
        CHECK (char_length(btrim(smtp_host)) > 0);

ALTER TABLE backup_history
    ADD COLUMN encrypted BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE backup_recipients (
    id               SERIAL PRIMARY KEY,
    email            VARCHAR(254) NOT NULL,
    name             VARCHAR(100),
    active           BOOLEAN      NOT NULL DEFAULT TRUE,
    send_attachment  BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT ck_backup_recipients_email
        CHECK (email = lower(btrim(email)) AND position('@' in email) > 1)
);

CREATE UNIQUE INDEX uq_backup_recipients_email ON backup_recipients (email);

CREATE TRIGGER trg_backup_recipients_updated_at
    BEFORE UPDATE ON backup_recipients
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Un intento de envio por destinatario (el correo se guarda como texto,
-- asi el registro sobrevive si luego se borra el destinatario).
CREATE TABLE backup_deliveries (
    id               SERIAL PRIMARY KEY,
    backup_id        INT          REFERENCES backup_history(id) ON DELETE RESTRICT,
    notice           VARCHAR(10)  NOT NULL,
    recipient_email  VARCHAR(254) NOT NULL,
    attached         BOOLEAN      NOT NULL DEFAULT FALSE,
    status           VARCHAR(10)  NOT NULL,
    error_message    TEXT,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT ck_backup_deliveries_notice CHECK (notice IN ('success', 'failure', 'test')),
    CONSTRAINT ck_backup_deliveries_status CHECK (status IN ('sent', 'failed'))
);

CREATE INDEX idx_backup_deliveries_backup ON backup_deliveries (backup_id);
CREATE INDEX idx_backup_deliveries_notice ON backup_deliveries (notice, created_at DESC);
CREATE INDEX idx_backup_deliveries_recent ON backup_deliveries (created_at DESC);

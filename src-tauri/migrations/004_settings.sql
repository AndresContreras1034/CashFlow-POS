-- ============================================================
-- MIGRACIÓN 004: Ajustes del negocio (fila única / singleton)
-- ============================================================

CREATE TABLE IF NOT EXISTS app_settings (
    id                          SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),

    business_name               VARCHAR(200) NOT NULL DEFAULT 'Mi Negocio',
    tax_id                      VARCHAR(50),
    address                     TEXT,
    phone                       VARCHAR(50),
    email                       VARCHAR(150),

    currency                    VARCHAR(3) NOT NULL DEFAULT 'COP',

    -- IVA en basis points: 1900 = 19.00%
    tax_rate_bps                INT NOT NULL DEFAULT 1900
                                CHECK (tax_rate_bps BETWEEN 0 AND 10000),

    ticket_header               TEXT,
    ticket_footer               TEXT,

    low_stock_default_threshold INT NOT NULL DEFAULT 5
                                CHECK (low_stock_default_threshold >= 0),

    logo_url                    TEXT,

    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Si el trigger ya existe porque la migración fue ejecutada
-- manualmente anteriormente, lo eliminamos antes de crearlo.
DROP TRIGGER IF EXISTS trg_settings_updated_at ON app_settings;

CREATE TRIGGER trg_settings_updated_at
    BEFORE UPDATE ON app_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Sembrar la única fila que va a existir
INSERT INTO app_settings (id)
VALUES (1)
ON CONFLICT (id) DO NOTHING;
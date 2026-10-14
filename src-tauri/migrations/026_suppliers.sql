-- ============================================================
-- MIGRACIÓN 026: Proveedores
-- ============================================================
-- Los proveedores no se borran, solo se desactivan (igual que clientes).
-- Sin FK hacia deudas: el cruce con deudas se hace en la etapa 6.
-- priority: 'normal' | 'preferred' | 'urgent' (marcado a mano).

CREATE TABLE suppliers (
    id                  SERIAL PRIMARY KEY,
    name                VARCHAR(150) NOT NULL,
    contact_name        VARCHAR(150),
    document_type       VARCHAR(10),
    document_number     VARCHAR(30),
    phone               VARCHAR(50),
    whatsapp            VARCHAR(50),
    email               VARCHAR(150),
    address             TEXT,
    city                VARCHAR(100),
    payment_terms_days  INT,
    lead_time_days      INT,
    visit_days          VARCHAR(100),
    priority            VARCHAR(10)  NOT NULL DEFAULT 'normal',
    notes               TEXT,
    is_active           BOOLEAN      NOT NULL DEFAULT TRUE,
    created_by          VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT ck_suppliers_name_not_blank
        CHECK (char_length(btrim(name)) > 0),
    CONSTRAINT ck_suppliers_document_pair
        CHECK ((document_type IS NULL) = (document_number IS NULL)),
    CONSTRAINT ck_suppliers_priority
        CHECK (priority IN ('normal', 'preferred', 'urgent')),
    CONSTRAINT ck_suppliers_payment_terms
        CHECK (payment_terms_days IS NULL OR payment_terms_days BETWEEN 0 AND 365),
    CONSTRAINT ck_suppliers_lead_time
        CHECK (lead_time_days IS NULL OR lead_time_days BETWEEN 0 AND 365)
);

-- Un mismo nombre no puede repetirse (también protege el cruce con deudas).
CREATE UNIQUE INDEX uq_suppliers_name
    ON suppliers (lower(btrim(name)));

-- Un mismo documento no puede repetirse entre proveedores.
CREATE UNIQUE INDEX uq_suppliers_document
    ON suppliers (document_type, document_number)
    WHERE document_number IS NOT NULL;

CREATE INDEX idx_suppliers_active   ON suppliers (is_active);
CREATE INDEX idx_suppliers_priority ON suppliers (priority) WHERE is_active;
CREATE INDEX idx_suppliers_city     ON suppliers (lower(city)) WHERE city IS NOT NULL;

CREATE TRIGGER trg_suppliers_updated_at
    BEFORE UPDATE ON suppliers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE FUNCTION prevent_supplier_deletion()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Los proveedores no se borran: desactívalo en su lugar'
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_suppliers_no_delete
    BEFORE DELETE ON suppliers
    FOR EACH ROW EXECUTE FUNCTION prevent_supplier_deletion();
CREATE TRIGGER trg_suppliers_no_truncate
    BEFORE TRUNCATE ON suppliers
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_supplier_deletion();

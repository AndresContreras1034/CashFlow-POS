-- ============================================================
-- MIGRACIÓN 024: Clientes + FK en ventas
-- ============================================================
-- Los clientes no se borran, solo se desactivan.
-- sales.customer_id NULL = consumidor final.

CREATE TABLE customers (
    id               SERIAL PRIMARY KEY,
    name             VARCHAR(150) NOT NULL,
    document_type    VARCHAR(10),
    document_number  VARCHAR(30),
    phone            VARCHAR(50),
    email            VARCHAR(150),
    address          TEXT,
    notes            TEXT,
    is_active        BOOLEAN      NOT NULL DEFAULT TRUE,
    created_by       VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT ck_customers_name_not_blank
        CHECK (char_length(btrim(name)) > 0),
    CONSTRAINT ck_customers_document_pair
        CHECK ((document_type IS NULL) = (document_number IS NULL))
);

-- Un mismo documento no puede repetirse entre clientes.
CREATE UNIQUE INDEX uq_customers_document
    ON customers (document_type, document_number)
    WHERE document_number IS NOT NULL;

CREATE INDEX idx_customers_name   ON customers (lower(name));
CREATE INDEX idx_customers_phone  ON customers (phone) WHERE phone IS NOT NULL;
CREATE INDEX idx_customers_active ON customers (is_active);

CREATE TRIGGER trg_customers_updated_at
    BEFORE UPDATE ON customers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE FUNCTION prevent_customer_deletion()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Los clientes no se borran: desactívalo en su lugar'
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_customers_no_delete
    BEFORE DELETE ON customers
    FOR EACH ROW EXECUTE FUNCTION prevent_customer_deletion();
CREATE TRIGGER trg_customers_no_truncate
    BEFORE TRUNCATE ON customers
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_customer_deletion();

-- Cerramos la FK que quedó reservada en la migración 006.
-- NOT VALID: no revisa filas históricas; las nuevas y modificadas sí cumplen.
ALTER TABLE sales
    ADD CONSTRAINT fk_sales_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE RESTRICT
    NOT VALID;

CREATE INDEX idx_sales_customer ON sales (customer_id)
    WHERE customer_id IS NOT NULL;

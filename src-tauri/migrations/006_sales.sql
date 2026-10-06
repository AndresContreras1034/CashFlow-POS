-- ============================================================
-- MIGRACIÓN 006: Ventas (venta + ítems + pagos mixtos)
-- ============================================================

CREATE TYPE payment_method AS ENUM ('cash', 'card', 'transfer');
CREATE TYPE sale_status    AS ENUM ('completed', 'cancelled', 'refunded');

CREATE TABLE sales (
    id           SERIAL PRIMARY KEY,

    -- FK futura cuando exista modules::customers (mismo criterio que
    -- inventory_movements.sale_id quedó reservado antes de que sales existiera).
    customer_id  INT,

    subtotal     BIGINT NOT NULL CHECK (subtotal >= 0),
    tax          BIGINT NOT NULL DEFAULT 0 CHECK (tax >= 0),
    discount     BIGINT NOT NULL DEFAULT 0 CHECK (discount >= 0),
    total        BIGINT NOT NULL CHECK (total >= 0),

    status       sale_status NOT NULL DEFAULT 'completed',
    notes        TEXT,
    created_by   VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sale_items (
    id          SERIAL PRIMARY KEY,
    sale_id     INT NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
    variant_id  INT NOT NULL REFERENCES product_variants(id) ON DELETE RESTRICT,

    quantity    INT NOT NULL CHECK (quantity > 0),
    unit_price  BIGINT NOT NULL CHECK (unit_price >= 0),
    discount    BIGINT NOT NULL DEFAULT 0 CHECK (discount >= 0),
    tax         BIGINT NOT NULL DEFAULT 0 CHECK (tax >= 0),
    subtotal    BIGINT NOT NULL CHECK (subtotal >= 0)
);

CREATE TABLE sale_payments (
    id       SERIAL PRIMARY KEY,
    sale_id  INT NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
    method   payment_method NOT NULL,
    amount   BIGINT NOT NULL CHECK (amount > 0)
);

CREATE INDEX idx_sale_items_sale      ON sale_items (sale_id);
CREATE INDEX idx_sale_items_variant   ON sale_items (variant_id);
CREATE INDEX idx_sale_payments_sale   ON sale_payments (sale_id);
CREATE INDEX idx_sales_status         ON sales (status);
CREATE INDEX idx_sales_created_at     ON sales (created_at);

-- Ahora que sales existe, cerramos las FKs que quedaron reservadas
-- como columnas sueltas en migraciones anteriores.
ALTER TABLE inventory_movements
    ADD CONSTRAINT fk_inventory_movements_sale
    FOREIGN KEY (sale_id) REFERENCES sales(id);

ALTER TABLE cash_movements
    ADD CONSTRAINT fk_cash_movements_sale
    FOREIGN KEY (sale_id) REFERENCES sales(id);
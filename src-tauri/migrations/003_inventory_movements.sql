-- ============================================================
-- MIGRACIÓN 003: Movimientos de Inventario (Kardex)
-- ============================================================

-- Tipos de movimiento posibles
CREATE TYPE movement_type AS ENUM (
    'purchase',         -- entrada por compra a proveedor
    'sale',             -- salida por venta (lo inserta el módulo de ventas)
    'sale_return',      -- entrada por devolución de venta
    'manual_in',        -- entrada manual (encontró unidades extra)
    'manual_out',       -- salida manual (daño, pérdida, vencimiento)
    'adjustment',       -- ajuste por conteo físico
    'initial_stock'     -- carga inicial de inventario
);

CREATE TABLE IF NOT EXISTS inventory_movements (
    id              SERIAL PRIMARY KEY,
    variant_id      INT NOT NULL REFERENCES product_variants(id) ON DELETE RESTRICT,

    movement_type   movement_type NOT NULL,

    -- Cantidad: siempre positiva. El tipo define si suma o resta.
    quantity        INT NOT NULL CHECK (quantity > 0),

    -- Stock antes y después del movimiento (para auditoría / Kardex)
    stock_before    INT NOT NULL,
    stock_after     INT NOT NULL,

    -- Costo unitario en el momento del movimiento (para valorización)
    unit_cost       BIGINT NOT NULL DEFAULT 0,

    -- Referencias opcionales
    sale_id         INT,        -- FK a sales cuando esté disponible
    purchase_id     INT,        -- FK a purchases cuando esté disponible

    notes           TEXT,
    created_by      VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_movements_variant   ON inventory_movements(variant_id);
CREATE INDEX idx_movements_type      ON inventory_movements(movement_type);
CREATE INDEX idx_movements_date      ON inventory_movements(created_at DESC);
CREATE INDEX idx_movements_sale      ON inventory_movements(sale_id) WHERE sale_id IS NOT NULL;

-- ------------------------------------------------------------
-- Función que actualiza updated_at automáticamente
-- (aplica a products y product_variants)
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_variants_updated_at
    BEFORE UPDATE ON product_variants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_categories_updated_at
    BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
-- ============================================================
-- MIGRACIÓN 032: vínculos proveedor–categoría y proveedor–variante
-- ============================================================
-- El brief de la etapa 2 pedía 027_supplier_links.sql. En este repo 027–031
-- ya están ocupados (impresora y backup), así que el esquema de vínculos
-- queda en 032 para no reordenar migraciones ajenas.

CREATE TABLE supplier_categories (
    supplier_id INT NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    category_id INT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (supplier_id, category_id)
);

CREATE INDEX idx_supplier_categories_category
    ON supplier_categories (category_id);

CREATE TABLE supplier_variants (
    id SERIAL PRIMARY KEY,
    supplier_id INT NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    variant_id INT NOT NULL REFERENCES product_variants(id) ON DELETE RESTRICT,
    supplier_sku VARCHAR(100),
    last_cost BIGINT CHECK (last_cost IS NULL OR last_cost >= 0),
    is_preferred BOOLEAN NOT NULL DEFAULT FALSE,
    last_purchase_at TIMESTAMPTZ,
    notes TEXT,
    created_by VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_supplier_variants_pair UNIQUE (supplier_id, variant_id)
);

CREATE UNIQUE INDEX uq_supplier_variants_preferred
    ON supplier_variants (variant_id)
    WHERE is_preferred;

CREATE INDEX idx_supplier_variants_variant
    ON supplier_variants (variant_id);

CREATE INDEX idx_supplier_variants_supplier
    ON supplier_variants (supplier_id);

CREATE TRIGGER trg_supplier_variants_updated_at
    BEFORE UPDATE ON supplier_variants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

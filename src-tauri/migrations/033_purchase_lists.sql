-- ============================================================
-- Listas de compras (panel de compras)
-- ============================================================
-- Estados: draft → ordered → received | cancelled. Las listas no se borran.
-- El costo se lee siempre de product_variants.cost.

CREATE TABLE purchase_lists (
    id            SERIAL PRIMARY KEY,
    title         VARCHAR(150) NOT NULL,
    notes         TEXT,
    status        VARCHAR(10)  NOT NULL DEFAULT 'draft',
    created_by    VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    ordered_at    TIMESTAMPTZ,
    received_at   TIMESTAMPTZ,
    cancelled_at  TIMESTAMPTZ,

    CONSTRAINT ck_purchase_lists_title_not_blank
        CHECK (char_length(btrim(title)) > 0),
    CONSTRAINT ck_purchase_lists_status
        CHECK (status IN ('draft', 'ordered', 'received', 'cancelled'))
);

CREATE INDEX idx_purchase_lists_status ON purchase_lists (status, updated_at DESC);

CREATE TRIGGER trg_purchase_lists_updated_at
    BEFORE UPDATE ON purchase_lists
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE FUNCTION prevent_purchase_list_deletion()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Las listas de compras no se borran: cancélala en su lugar'
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_purchase_lists_no_delete
    BEFORE DELETE ON purchase_lists
    FOR EACH ROW EXECUTE FUNCTION prevent_purchase_list_deletion();
CREATE TRIGGER trg_purchase_lists_no_truncate
    BEFORE TRUNCATE ON purchase_lists
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_purchase_list_deletion();

CREATE TABLE purchase_list_items (
    id             SERIAL PRIMARY KEY,
    list_id        INT NOT NULL REFERENCES purchase_lists(id) ON DELETE RESTRICT,
    variant_id     INT NOT NULL REFERENCES product_variants(id) ON DELETE RESTRICT,
    supplier_id    INT REFERENCES suppliers(id) ON DELETE RESTRICT,
    quantity       INT NOT NULL,
    source         VARCHAR(12) NOT NULL DEFAULT 'manual',
    suggested_qty  INT,
    note           TEXT,
    created_by     VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_purchase_list_items UNIQUE (list_id, variant_id),
    CONSTRAINT ck_purchase_list_items_quantity CHECK (quantity BETWEEN 1 AND 100000),
    CONSTRAINT ck_purchase_list_items_source CHECK (source IN ('recommended', 'manual')),
    CONSTRAINT ck_purchase_list_items_suggested
        CHECK (suggested_qty IS NULL OR suggested_qty > 0),
    CONSTRAINT ck_purchase_list_items_note
        CHECK (note IS NULL OR char_length(note) <= 500)
);

CREATE INDEX idx_purchase_list_items_variant ON purchase_list_items (variant_id);
CREATE INDEX idx_purchase_list_items_supplier ON purchase_list_items (supplier_id)
    WHERE supplier_id IS NOT NULL;

CREATE TRIGGER trg_purchase_list_items_updated_at
    BEFORE UPDATE ON purchase_list_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

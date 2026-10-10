ALTER TABLE sale_items
    ADD COLUMN unit_cost BIGINT NULL
    CHECK (unit_cost IS NULL OR unit_cost >= 0);

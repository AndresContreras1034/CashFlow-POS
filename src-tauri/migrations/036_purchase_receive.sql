ALTER TABLE purchase_list_items
    ADD COLUMN received_quantity INT,
    ADD COLUMN received_unit_cost BIGINT,
    ADD CONSTRAINT ck_purchase_items_received_qty
        CHECK (received_quantity IS NULL OR received_quantity BETWEEN 0 AND 100000),
    ADD CONSTRAINT ck_purchase_items_received_cost
        CHECK (received_unit_cost IS NULL OR received_unit_cost > 0);

CREATE TYPE movement_reason AS ENUM (
    'damaged',
    'lost',
    'expired',
    'theft',
    'internal_use',
    'count_correction',
    'other'
);

ALTER TABLE inventory_movements
    ADD COLUMN reason movement_reason;

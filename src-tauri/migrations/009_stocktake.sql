CREATE TYPE stocktake_status AS ENUM ('counting', 'applied', 'cancelled');

CREATE TABLE IF NOT EXISTS stocktakes (
    id           SERIAL PRIMARY KEY,
    category_id  INT REFERENCES categories(id) ON DELETE RESTRICT,
    status       stocktake_status NOT NULL DEFAULT 'counting',
    notes        TEXT,
    created_by   VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied_at   TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX one_open_stocktake
    ON stocktakes ((status))
    WHERE status = 'counting';

CREATE TABLE IF NOT EXISTS stocktake_lines (
    id             SERIAL PRIMARY KEY,
    stocktake_id   INT NOT NULL REFERENCES stocktakes(id) ON DELETE CASCADE,
    variant_id     INT NOT NULL REFERENCES product_variants(id) ON DELETE RESTRICT,
    expected_stock INT NOT NULL,
    counted_stock  INT CHECK (counted_stock >= 0),
    counted_at     TIMESTAMPTZ,
    UNIQUE (stocktake_id, variant_id)
);

CREATE INDEX idx_stocktake_lines_session ON stocktake_lines(stocktake_id);

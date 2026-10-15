ALTER TABLE stocktake_lines
    ADD COLUMN stock_at_count INT;

UPDATE stocktake_lines
SET stock_at_count = expected_stock
WHERE stock_at_count IS NULL;

ALTER TABLE stocktake_lines
    ALTER COLUMN stock_at_count SET NOT NULL;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM product_variants
        WHERE stock < 0 AND NOT allow_negative
    ) THEN
        RAISE EXCEPTION 'Cannot validate stock policy: negative stock exists while allow_negative is false';
    END IF;
    IF EXISTS (
        SELECT 1 FROM product_variants
        WHERE stock_min < 0
    ) THEN
        RAISE EXCEPTION 'Cannot validate stock_min policy: negative stock_min exists';
    END IF;
END
$$;

ALTER TABLE product_variants
    ADD CONSTRAINT product_variants_stock_policy
    CHECK (allow_negative OR stock >= 0) NOT VALID;

ALTER TABLE product_variants
    VALIDATE CONSTRAINT product_variants_stock_policy;

ALTER TABLE product_variants
    ADD CONSTRAINT product_variants_stock_min_nonnegative
    CHECK (stock_min >= 0) NOT VALID;

ALTER TABLE product_variants
    VALIDATE CONSTRAINT product_variants_stock_min_nonnegative;

CREATE OR REPLACE FUNCTION debt_today()
RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT (NOW() AT TIME ZONE COALESCE(
        (
            SELECT s.timezone
            FROM app_settings s
            WHERE s.id = 1
              AND EXISTS (
                  SELECT 1 FROM pg_timezone_names z WHERE z.name = s.timezone
              )
        ),
        'UTC'
    ))::DATE
$$;

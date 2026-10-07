-- Extend business settings for currency, tax labels, timezone, and ticket output.
ALTER TABLE app_settings
    ADD COLUMN currency_decimals   SMALLINT    NOT NULL DEFAULT 0
                                   CHECK (currency_decimals IN (0, 2)),
    ADD COLUMN tax_name            VARCHAR(30) NOT NULL DEFAULT 'IVA'
                                   CHECK (char_length(btrim(tax_name)) > 0),
    ADD COLUMN timezone            VARCHAR(64) NOT NULL DEFAULT 'America/Bogota',
    ADD COLUMN show_logo           BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_tax_id         BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_address        BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_phone          BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_cashier        BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_tax_breakdown  BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_discounts      BOOLEAN     NOT NULL DEFAULT TRUE,
    ADD COLUMN show_payment_method BOOLEAN     NOT NULL DEFAULT TRUE;

UPDATE app_settings
SET currency_decimals = CASE WHEN currency = 'COP' THEN 0 ELSE 2 END
WHERE id = 1;

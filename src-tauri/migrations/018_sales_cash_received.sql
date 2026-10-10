ALTER TABLE sales ADD COLUMN cash_received BIGINT, ADD COLUMN change_given BIGINT;

ALTER TABLE sales ADD CONSTRAINT sales_cash_change_chk CHECK (
    (cash_received IS NULL AND change_given IS NULL)
    OR (cash_received IS NOT NULL AND change_given IS NOT NULL AND total > 0
        AND change_given >= 0 AND cash_received > change_given)
);

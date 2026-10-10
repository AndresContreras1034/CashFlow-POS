ALTER TABLE sales ADD COLUMN courtesy_reason TEXT;

ALTER TABLE sales ADD CONSTRAINT sales_courtesy_reason_chk CHECK (
    (total <> 0 AND courtesy_reason IS NULL)
    OR (total = 0
        AND courtesy_reason IS NOT NULL
        AND btrim(courtesy_reason) <> ''
        AND char_length(courtesy_reason) <= 200)
);

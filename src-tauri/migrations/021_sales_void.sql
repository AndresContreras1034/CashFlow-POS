-- NOT VALID conserva las filas históricas sin revisarlas al crear el CHECK;
-- las nuevas y modificadas sí deben cumplirlo. Validar históricos tras revisarlos.
ALTER TABLE sales
    ADD COLUMN voided_at   TIMESTAMPTZ,
    ADD COLUMN void_reason TEXT,
    ADD COLUMN voided_by   TEXT;

ALTER TABLE sales ADD CONSTRAINT sales_void_consistency_chk CHECK (
    (status = 'completed' AND voided_at IS NULL AND void_reason IS NULL AND voided_by IS NULL)
    OR (status <> 'completed' AND voided_at IS NOT NULL AND voided_by IS NOT NULL
        AND void_reason IS NOT NULL AND btrim(void_reason) <> ''
        AND char_length(void_reason) <= 200)
) NOT VALID;

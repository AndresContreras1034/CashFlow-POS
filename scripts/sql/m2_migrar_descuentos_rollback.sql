-- Rollback documentado: ejecutar solo si existe sales_m2_backup.
BEGIN;
UPDATE sales s SET subtotal = b.subtotal, discount = b.discount, gross_semantics = b.gross_semantics
FROM sales_m2_backup b WHERE b.id = s.id;
COMMIT;

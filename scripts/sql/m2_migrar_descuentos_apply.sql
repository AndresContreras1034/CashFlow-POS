-- M2 apply. Ejecutar solo tras respaldo y dry-run revisado:
--   psql -v ON_ERROR_STOP=1 -h localhost -p 5432 -U postgres -d pos_db -f scripts/sql/m2_migrar_descuentos_apply.sql
BEGIN;

-- Copia permanente para rollback; si ya existe, el script falla y no se aplica dos veces.
CREATE TABLE sales_m2_backup AS
    SELECT id, subtotal, discount, gross_semantics FROM sales WHERE NOT gross_semantics;

DO $$
DECLARE bad INT;
BEGIN
    SELECT count(*) INTO bad FROM (
        SELECT s.id FROM sales s LEFT JOIN sale_items si ON si.sale_id = s.id
        WHERE NOT s.gross_semantics
        GROUP BY s.id, s.subtotal, s.discount, s.total
        HAVING s.subtotal <> COALESCE(SUM(si.subtotal),0) OR s.subtotal - s.discount <> s.total
    ) x;
    IF bad > 0 THEN
        RAISE EXCEPTION 'M2: % ventas incumplen el invariante previo; no se aplica nada', bad;
    END IF;
END $$;

UPDATE sales s
SET subtotal = s.subtotal + x.d, discount = s.discount + x.d
FROM (SELECT sale_id, SUM(discount)::BIGINT AS d FROM sale_items
      GROUP BY sale_id HAVING SUM(discount) > 0) x
WHERE x.sale_id = s.id AND NOT s.gross_semantics;

UPDATE sales SET gross_semantics = TRUE WHERE NOT gross_semantics;

DO $$
DECLARE bad INT;
BEGIN
    SELECT count(*) INTO bad FROM sales s
    LEFT JOIN (SELECT sale_id, SUM(unit_price*quantity)::BIGINT AS g FROM sale_items GROUP BY sale_id) i
      ON i.sale_id = s.id
    WHERE s.subtotal - s.discount <> s.total OR s.subtotal <> COALESCE(i.g, 0);
    IF bad > 0 THEN
        RAISE EXCEPTION 'M2: % ventas incumplen el invariante posterior; se revierte', bad;
    END IF;
END $$;

COMMIT;
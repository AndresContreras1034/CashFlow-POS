-- ============================================================
-- MIGRACIÓN 015: Idempotencia de ventas
-- ============================================================
-- Clave opcional generada por el cliente por cada intento de venta.
-- Nullable: las ventas existentes y los clientes que no la envían
-- siguen funcionando. El índice único parcial es la garantía final
-- en la base de datos.
ALTER TABLE sales ADD COLUMN idempotency_key UUID;

CREATE UNIQUE INDEX uq_sales_idempotency_key
    ON sales (idempotency_key)
    WHERE idempotency_key IS NOT NULL;
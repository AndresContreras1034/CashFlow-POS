-- ============================================================
-- MIGRACIÓN 013: Módulo de auditoría «debts»
-- ============================================================
--
-- Va sola a propósito: sqlx ejecuta cada migración dentro de una
-- transacción y PostgreSQL no permite usar un valor de enum recién
-- añadido hasta que esa transacción termina. La migración 014 ya
-- puede usar 'debts'.
-- ============================================================

ALTER TYPE audit_module ADD VALUE IF NOT EXISTS 'debts';
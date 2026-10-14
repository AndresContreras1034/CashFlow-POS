-- Va sola a proposito (igual que 013, 023 y 025): PostgreSQL no permite usar un
-- valor de enum recien anadido dentro de la misma transaccion.
ALTER TYPE audit_module ADD VALUE IF NOT EXISTS 'backup';

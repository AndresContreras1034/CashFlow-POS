-- Va sola a propósito (igual que 013 y 023): PostgreSQL no permite usar un
-- valor de enum recién añadido dentro de la misma transacción.
ALTER TYPE audit_module ADD VALUE IF NOT EXISTS 'suppliers';

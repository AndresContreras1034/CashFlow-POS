-- ============================================================
-- MIGRACIÓN 001: Categorías
-- ============================================================

CREATE TABLE IF NOT EXISTS categories (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Categorías base para una tienda de detalles
INSERT INTO categories (name, description) VALUES
    ('Perfumes y Fragancias', 'Colonias, perfumes y desodorantes'),
    ('Joyería y Accesorios',  'Aretes, collares, pulseras, anillos'),
    ('Juguetes',              'Juguetes para niños de todas las edades'),
    ('Ropa',                  'Prendas de vestir para adultos y niños'),
    ('Detalles y Regalos',    'Artículos de regalo y decoración'),
    ('Bisutería',             'Accesorios de moda a bajo costo'),
    ('Cosméticos',            'Maquillaje y cuidado personal'),
    ('Sin categoría',         'Productos sin categoría asignada')
ON CONFLICT (name) DO NOTHING;
-- ============================================================
-- MIGRACIÓN 002: Productos y Variantes
-- ============================================================

-- ------------------------------------------------------------
-- Tabla padre: información general del producto
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS products (
    id           SERIAL PRIMARY KEY,
    category_id  INT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    name         VARCHAR(200) NOT NULL,
    description  TEXT,
    brand        VARCHAR(100),
    image_url    TEXT,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_products_category  ON products(category_id);
CREATE INDEX idx_products_is_active ON products(is_active);
CREATE INDEX idx_products_name      ON products USING gin(to_tsvector('spanish', name));

-- ------------------------------------------------------------
-- Tabla de variantes: cada combinación única de un producto
-- Ejemplos:
--   Perfume "Acqua di Gio" → variante 50ml, variante 100ml
--   Camiseta "Básica"      → variante Talla S, variante Talla M
--   Juguete "Carro"        → variante Rojo, variante Azul
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS product_variants (
    id             SERIAL PRIMARY KEY,
    product_id     INT NOT NULL REFERENCES products(id) ON DELETE CASCADE,

    -- Atributos de la variante (ej: "100ml", "Talla M", "Color Rojo")
    -- Se almacenan como pares clave-valor en JSONB para máxima flexibilidad
    -- Ejemplo: {"talla": "M", "color": "Rojo"} | {"volumen": "100ml"} | {"presentacion": "Unidad"}
    attributes     JSONB NOT NULL DEFAULT '{}',

    -- Identificación
    sku            VARCHAR(100) UNIQUE,              -- código interno opcional
    barcode        VARCHAR(100) UNIQUE,              -- código de barras del empaque

    -- Precio en centavos para evitar errores de punto flotante
    -- Ejemplo: $45.000 COP → 4500000
    price          BIGINT NOT NULL CHECK (price >= 0),
    cost           BIGINT NOT NULL DEFAULT 0 CHECK (cost >= 0),  -- precio de compra

    -- Stock
    stock          INT NOT NULL DEFAULT 0,
    stock_min      INT NOT NULL DEFAULT 0,           -- alerta de bajo stock
    allow_negative BOOLEAN NOT NULL DEFAULT FALSE,   -- ¿vender sin stock?

    is_active      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- No permitir dos variantes idénticas para el mismo producto
    CONSTRAINT uq_product_attributes UNIQUE (product_id, attributes)
);

CREATE INDEX idx_variants_product    ON product_variants(product_id);
CREATE INDEX idx_variants_barcode    ON product_variants(barcode);
CREATE INDEX idx_variants_sku        ON product_variants(sku);
CREATE INDEX idx_variants_is_active  ON product_variants(is_active);
CREATE INDEX idx_variants_stock_low  ON product_variants(stock) WHERE stock > 0;
CREATE INDEX idx_variants_attributes ON product_variants USING gin(attributes);
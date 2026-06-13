use sqlx::PgPool;

use crate::modules::inventory::{
    dto::{
        CreateCategoryDto, CreateProductDto, CreateVariantDto, KardexFilterDto,
        ProductFilterDto, StockAdjustmentDto, StockEntryDto, StockOutDto, UpdateCategoryDto,
        UpdateProductDto, UpdateVariantDto,
    },
    models::{
        Category, InventoryMovement, MovementType, MovementWithDetails, Product,
        ProductVariant, ProductWithCategory, VariantWithProduct,
    },
};

// ============================================================
// CATEGORÍAS
// ============================================================

pub async fn get_all_categories(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as!(
        Category,
        "SELECT * FROM categories WHERE is_active = TRUE ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_category_by_id(pool: &PgPool, id: i32) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as!(Category, "SELECT * FROM categories WHERE id = $1", id)
        .fetch_optional(pool)
        .await
}

pub async fn create_category(
    pool: &PgPool,
    dto: CreateCategoryDto,
) -> Result<Category, sqlx::Error> {
    sqlx::query_as!(
        Category,
        "INSERT INTO categories (name, description)
         VALUES ($1, $2)
         RETURNING *",
        dto.name,
        dto.description
    )
    .fetch_one(pool)
    .await
}

pub async fn update_category(
    pool: &PgPool,
    id: i32,
    dto: UpdateCategoryDto,
) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as!(
        Category,
        "UPDATE categories
         SET name        = COALESCE($1, name),
             description = COALESCE($2, description),
             is_active   = COALESCE($3, is_active)
         WHERE id = $4
         RETURNING *",
        dto.name,
        dto.description,
        dto.is_active,
        id
    )
    .fetch_optional(pool)
    .await
}

// ============================================================
// PRODUCTOS
// ============================================================

pub async fn get_products(
    pool: &PgPool,
    filter: &ProductFilterDto,
) -> Result<(Vec<ProductWithCategory>, i64), sqlx::Error> {
    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset    = (page - 1) * page_size;
    let search    = filter.search.as_deref().map(|s| format!("%{}%", s));
    let is_active = filter.is_active.unwrap_or(true);

    let rows = sqlx::query_as!(
        ProductWithCategory,
        r#"SELECT
               p.id, p.category_id, c.name AS category_name,
               p.name, p.description, p.brand, p.image_url,
               p.is_active, p.created_at, p.updated_at
           FROM products p
           JOIN categories c ON c.id = p.category_id
           WHERE p.is_active = $1
             AND ($2::INT  IS NULL OR p.category_id = $2)
             AND ($3::TEXT IS NULL OR p.name ILIKE $3 OR p.brand ILIKE $3)
           ORDER BY p.name ASC
           LIMIT $4 OFFSET $5"#,
        is_active,
        filter.category_id,
        search,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM products p
         WHERE p.is_active = $1
           AND ($2::INT  IS NULL OR p.category_id = $2)
           AND ($3::TEXT IS NULL OR p.name ILIKE $3 OR p.brand ILIKE $3)",
        is_active,
        filter.category_id,
        search
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

pub async fn get_product_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<ProductWithCategory>, sqlx::Error> {
    sqlx::query_as!(
        ProductWithCategory,
        r#"SELECT
               p.id, p.category_id, c.name AS category_name,
               p.name, p.description, p.brand, p.image_url,
               p.is_active, p.created_at, p.updated_at
           FROM products p
           JOIN categories c ON c.id = p.category_id
           WHERE p.id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn create_product(
    pool: &PgPool,
    dto: CreateProductDto,
) -> Result<Product, sqlx::Error> {
    sqlx::query_as!(
        Product,
        "INSERT INTO products (category_id, name, description, brand, image_url)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
        dto.category_id,
        dto.name,
        dto.description,
        dto.brand,
        dto.image_url
    )
    .fetch_one(pool)
    .await
}

pub async fn update_product(
    pool: &PgPool,
    id: i32,
    dto: UpdateProductDto,
) -> Result<Option<Product>, sqlx::Error> {
    sqlx::query_as!(
        Product,
        "UPDATE products
         SET category_id = COALESCE($1, category_id),
             name        = COALESCE($2, name),
             description = COALESCE($3, description),
             brand       = COALESCE($4, brand),
             image_url   = COALESCE($5, image_url),
             is_active   = COALESCE($6, is_active)
         WHERE id = $7
         RETURNING *",
        dto.category_id,
        dto.name,
        dto.description,
        dto.brand,
        dto.image_url,
        dto.is_active,
        id
    )
    .fetch_optional(pool)
    .await
}

// ============================================================
// VARIANTES
// ============================================================

pub async fn get_variants_by_product(
    pool: &PgPool,
    product_id: i32,
) -> Result<Vec<ProductVariant>, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "SELECT * FROM product_variants WHERE product_id = $1 ORDER BY id ASC",
        product_id
    )
    .fetch_all(pool)
    .await
}

pub async fn get_variant_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<ProductVariant>, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "SELECT * FROM product_variants WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await
}

/// Busca variante por código de barras — usado en ventas al escanear
pub async fn get_variant_by_barcode(
    pool: &PgPool,
    barcode: &str,
) -> Result<Option<VariantWithProduct>, sqlx::Error> {
    sqlx::query_as!(
        VariantWithProduct,
        r#"SELECT
               v.id, v.product_id, v.attributes, v.sku, v.barcode,
               v.price, v.cost, v.stock, v.stock_min, v.allow_negative,
               v.is_active,
               p.name AS product_name, p.brand, p.image_url,
               p.category_id, c.name AS category_name
           FROM product_variants v
           JOIN products   p ON p.id = v.product_id
           JOIN categories c ON c.id = p.category_id
           WHERE v.barcode = $1 AND v.is_active = TRUE AND p.is_active = TRUE"#,
        barcode
    )
    .fetch_optional(pool)
    .await
}

/// Busca variantes por texto — usado en ventas cuando no hay barcode
pub async fn search_variants(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<VariantWithProduct>, sqlx::Error> {
    let pattern = format!("%{}%", query);
    sqlx::query_as!(
        VariantWithProduct,
        r#"SELECT
               v.id, v.product_id, v.attributes, v.sku, v.barcode,
               v.price, v.cost, v.stock, v.stock_min, v.allow_negative,
               v.is_active,
               p.name AS product_name, p.brand, p.image_url,
               p.category_id, c.name AS category_name
           FROM product_variants v
           JOIN products   p ON p.id = v.product_id
           JOIN categories c ON c.id = p.category_id
           WHERE v.is_active = TRUE AND p.is_active = TRUE
             AND (p.name ILIKE $1 OR p.brand ILIKE $1 OR v.sku ILIKE $1 OR v.barcode ILIKE $1)
           ORDER BY p.name ASC
           LIMIT 20"#,
        pattern
    )
    .fetch_all(pool)
    .await
}

pub async fn create_variant(
    pool: &PgPool,
    dto: CreateVariantDto,
) -> Result<ProductVariant, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "INSERT INTO product_variants
             (product_id, attributes, sku, barcode, price, cost, stock, stock_min, allow_negative)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING *",
        dto.product_id,
        dto.attributes,
        dto.sku,
        dto.barcode,
        dto.price,
        dto.cost.unwrap_or(0),
        dto.stock.unwrap_or(0),
        dto.stock_min.unwrap_or(0),
        dto.allow_negative.unwrap_or(false)
    )
    .fetch_one(pool)
    .await
}

pub async fn update_variant(
    pool: &PgPool,
    id: i32,
    dto: UpdateVariantDto,
) -> Result<Option<ProductVariant>, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants
         SET attributes     = COALESCE($1, attributes),
             sku            = COALESCE($2, sku),
             barcode        = COALESCE($3, barcode),
             price          = COALESCE($4, price),
             cost           = COALESCE($5, cost),
             stock_min      = COALESCE($6, stock_min),
             allow_negative = COALESCE($7, allow_negative),
             is_active      = COALESCE($8, is_active)
         WHERE id = $9
         RETURNING *",
        dto.attributes,
        dto.sku,
        dto.barcode,
        dto.price,
        dto.cost,
        dto.stock_min,
        dto.allow_negative,
        dto.is_active,
        id
    )
    .fetch_optional(pool)
    .await
}

// ============================================================
// STOCK — operaciones atómicas con movimiento incluido
// ============================================================

/// Aplica entrada de stock y registra movimiento. Retorna variante actualizada.
pub async fn apply_stock_entry(
    pool: &PgPool,
    dto: StockEntryDto,
    movement_type: MovementType,
) -> Result<ProductVariant, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Leer stock actual con lock para evitar race conditions
    let current = sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let stock_before = current;
    let stock_after  = current + dto.quantity;

    // 2. Actualizar stock
    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
        stock_after,
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // 3. Registrar movimiento
    sqlx::query!(
        "INSERT INTO inventory_movements
             (variant_id, movement_type, quantity, stock_before, stock_after, unit_cost, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        dto.variant_id,
        movement_type as MovementType,
        dto.quantity,
        stock_before,
        stock_after,
        dto.unit_cost.unwrap_or(0),
        dto.notes,
        dto.created_by.as_deref().unwrap_or("system")
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(variant)
}

/// Aplica salida de stock y registra movimiento.
pub async fn apply_stock_out(
    pool: &PgPool,
    dto: StockOutDto,
    movement_type: MovementType,
) -> Result<ProductVariant, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        "SELECT stock, allow_negative FROM product_variants WHERE id = $1 FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let stock_before = row.stock;
    let stock_after  = stock_before - dto.quantity;

    // Bloquear si no permite negativos
    if stock_after < 0 && !row.allow_negative {
        return Err(sqlx::Error::RowNotFound); // el service convierte esto en error amigable
    }

    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
        stock_after,
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO inventory_movements
             (variant_id, movement_type, quantity, stock_before, stock_after, unit_cost, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7)",
        dto.variant_id,
        movement_type as MovementType,
        dto.quantity,
        stock_before,
        stock_after,
        dto.notes,
        dto.created_by.as_deref().unwrap_or("system")
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(variant)
}

/// Ajuste por conteo físico — establece el stock al valor real contado.
pub async fn apply_stock_adjustment(
    pool: &PgPool,
    dto: StockAdjustmentDto,
) -> Result<ProductVariant, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let stock_before = sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let diff = (dto.actual_stock - stock_before).abs();

    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
        dto.actual_stock,
        dto.variant_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Solo registrar movimiento si hubo diferencia
    if diff > 0 {
        sqlx::query!(
            "INSERT INTO inventory_movements
                 (variant_id, movement_type, quantity, stock_before, stock_after, unit_cost, notes, created_by)
             VALUES ($1, 'adjustment', $2, $3, $4, 0, $5, $6)",
            dto.variant_id,
            diff,
            stock_before,
            dto.actual_stock,
            dto.notes,
            dto.created_by.as_deref().unwrap_or("system")
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(variant)
}

// ============================================================
// KARDEX
// ============================================================

pub async fn get_kardex(
    pool: &PgPool,
    filter: &KardexFilterDto,
) -> Result<(Vec<MovementWithDetails>, i64), sqlx::Error> {
    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);
    let offset    = (page - 1) * page_size;

    let rows = sqlx::query_as!(
        MovementWithDetails,
        r#"SELECT
               m.id, m.variant_id,
               m.movement_type AS "movement_type: MovementType",
               m.quantity, m.stock_before, m.stock_after,
               m.unit_cost, m.notes, m.created_by, m.created_at,
               p.name  AS product_name,
               v.attributes, v.sku, v.barcode
           FROM inventory_movements m
           JOIN product_variants v ON v.id = m.variant_id
           JOIN products         p ON p.id = v.product_id
           WHERE ($1::INT  IS NULL OR m.variant_id = $1)
             AND ($2::INT  IS NULL OR v.product_id = $2)
             AND ($3::TEXT IS NULL OR m.movement_type::TEXT = $3)
             AND ($4::DATE IS NULL OR m.created_at::DATE >= $4::DATE)
             AND ($5::DATE IS NULL OR m.created_at::DATE <= $5::DATE)
           ORDER BY m.created_at DESC
           LIMIT $6 OFFSET $7"#,
        filter.variant_id,
        filter.product_id,
        filter.movement_type,
        filter.date_from,
        filter.date_to,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM inventory_movements m
         JOIN product_variants v ON v.id = m.variant_id
         WHERE ($1::INT  IS NULL OR m.variant_id = $1)
           AND ($2::INT  IS NULL OR v.product_id = $2)
           AND ($3::TEXT IS NULL OR m.movement_type::TEXT = $3)
           AND ($4::DATE IS NULL OR m.created_at::DATE >= $4::DATE)
           AND ($5::DATE IS NULL OR m.created_at::DATE <= $5::DATE)",
        filter.variant_id,
        filter.product_id,
        filter.movement_type,
        filter.date_from,
        filter.date_to
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

// ============================================================
// ALERTAS DE STOCK
// ============================================================

pub async fn get_low_stock_variants(pool: &PgPool) -> Result<Vec<ProductVariant>, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "SELECT * FROM product_variants
         WHERE is_active = TRUE AND stock <= stock_min
         ORDER BY stock ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_out_of_stock_variants(pool: &PgPool) -> Result<Vec<ProductVariant>, sqlx::Error> {
    sqlx::query_as!(
        ProductVariant,
        "SELECT * FROM product_variants
         WHERE is_active = TRUE AND stock <= 0
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
}
use serde_json::{json, Map, Value as JsonValue};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::modules::audit::{
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};

/// Campos de fila que no representan cambios de negocio.
const AUDIT_IGNORED_FIELDS: [&str; 3] = ["id", "created_at", "updated_at"];

/// Construye eventos de catálogo de inventario sin actor mientras no exista login.
fn inventory_event(
    action: &str,
    entity_type: &str,
    entity_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: Option<JsonValue>,
) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id: Some(Uuid::new_v4()),
        category: AuditCategory::Business,
        module: AuditModule::Inventory,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor: None,
        entity_type: Some(entity_type.to_string()),
        entity_id: Some(entity_id.to_string()),
        summary,
        changes,
        metadata,
        error_message: None,
    }
}

fn field_list(changes: &Map<String, JsonValue>) -> String {
    changes
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

use crate::modules::inventory::{
    dto::{
        CreateCategoryDto, CreateProductDto, CreateVariantDto, ExportVariantRow, InventoryValueDto,
        KardexFilterDto, LabelData, LowStockItemDto, ProductFilterDto, ProductSortBy,
        ProductStockStatsDto, SortDir, StockAdjustmentDto, StockEntryDto, StockOutDto,
        UpdateCategoryDto, UpdateProductDto, UpdateVariantDto,
    },
    models::{
        Category, MovementReason, MovementType, MovementWithDetails, Product, ProductVariant,
        ProductWithCategory, VariantWithProduct,
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
    let mut tx = pool.begin().await?;

    let category = sqlx::query_as!(
        Category,
        "INSERT INTO categories (name, description)
         VALUES ($1, $2)
         RETURNING *",
        dto.name,
        dto.description
    )
    .fetch_one(&mut *tx)
    .await?;

    audit_repo::insert_event(
        &mut *tx,
        inventory_event(
            "create",
            "category",
            category.id,
            format!("Categoría #{} creada: {}", category.id, category.name),
            None,
            Some(json!({
                "name": category.name,
                "description": category.description,
            })),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(category)
}

pub async fn update_category(
    pool: &PgPool,
    id: i32,
    dto: UpdateCategoryDto,
) -> Result<Option<Category>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let before = sqlx::query_as!(
        Category,
        "SELECT * FROM categories WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(before) = before else {
        return Ok(None);
    };

    let after = sqlx::query_as!(
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
    .fetch_one(&mut *tx)
    .await?;

    let changes = changed_fields(&before, &after, &AUDIT_IGNORED_FIELDS);
    if !changes.is_empty() {
        let summary = format!(
            "Categoría #{} actualizada: {}",
            after.id,
            field_list(&changes)
        );
        audit_repo::insert_event(
            &mut *tx,
            inventory_event(
                "update",
                "category",
                after.id,
                summary,
                Some(JsonValue::Object(changes)),
                None,
            ),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(Some(after))
}

// ============================================================
// PRODUCTOS
// ============================================================

pub async fn get_products(
    pool: &PgPool,
    filter: &ProductFilterDto,
) -> Result<(Vec<ProductWithCategory>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let search = filter.search.as_deref().map(|s| format!("%{}%", s));
    let is_active = filter.is_active;
    let stock_status = filter.stock_status.map(|status| status.as_str());
    let sort_by = filter.sort_by.unwrap_or(ProductSortBy::Name).as_str();
    let sort_dir = filter.sort_dir.unwrap_or(SortDir::Asc).as_str();

    let rows = sqlx::query_as!(
        ProductWithCategory,
        r#"SELECT
               p.id, p.category_id, c.name AS category_name,
               p.name, p.description, p.brand, p.image_url,
               p.is_active, p.created_at, p.updated_at
           FROM products p
           JOIN categories c ON c.id = p.category_id
           WHERE ($1::BOOL IS NULL OR p.is_active = $1)
             AND ($2::INT  IS NULL OR p.category_id = $2)
             AND ($3::TEXT IS NULL
                  OR p.name ILIKE $3
                  OR p.brand ILIKE $3
                  OR EXISTS (
                      SELECT 1 FROM product_variants v
                      WHERE v.product_id = p.id
                        AND (v.sku ILIKE $3 OR v.barcode ILIKE $3)
                  ))
             AND ($4::BOOL IS NULL OR EXISTS (
                      SELECT 1 FROM product_variants v
                      WHERE v.product_id = p.id
                        AND v.is_active = $4
                  ))
             AND ($5::TEXT IS NULL OR EXISTS (
                      SELECT 1 FROM product_variants v
                      WHERE v.product_id = p.id
                        AND v.is_active = TRUE
                        AND CASE $5::TEXT
                              WHEN 'out_of_stock' THEN v.stock <= 0
                              WHEN 'low' THEN (v.stock > 0 AND v.stock <= v.stock_min)
                              WHEN 'ok' THEN v.stock > v.stock_min
                              ELSE FALSE
                            END
                  ))
           ORDER BY
               CASE WHEN $6::TEXT = 'name'       AND $7::TEXT = 'asc'  THEN p.name       END ASC,
               CASE WHEN $6::TEXT = 'name'       AND $7::TEXT = 'desc' THEN p.name       END DESC,
               CASE WHEN $6::TEXT = 'brand'      AND $7::TEXT = 'asc'  THEN p.brand      END ASC,
               CASE WHEN $6::TEXT = 'brand'      AND $7::TEXT = 'desc' THEN p.brand      END DESC,
               CASE WHEN $6::TEXT = 'category'   AND $7::TEXT = 'asc'  THEN c.name       END ASC,
               CASE WHEN $6::TEXT = 'category'   AND $7::TEXT = 'desc' THEN c.name       END DESC,
               CASE WHEN $6::TEXT = 'created_at' AND $7::TEXT = 'asc'  THEN p.created_at END ASC,
               CASE WHEN $6::TEXT = 'created_at' AND $7::TEXT = 'desc' THEN p.created_at END DESC,
               p.id ASC
           LIMIT $8 OFFSET $9"#,
        is_active,
        filter.category_id,
        search,
        filter.variant_is_active,
        stock_status,
        sort_by,
        sort_dir,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM products p
         WHERE ($1::BOOL IS NULL OR p.is_active = $1)
           AND ($2::INT  IS NULL OR p.category_id = $2)
           AND ($3::TEXT IS NULL
                OR p.name ILIKE $3
                OR p.brand ILIKE $3
                OR EXISTS (
                    SELECT 1 FROM product_variants v
                    WHERE v.product_id = p.id
                      AND (v.sku ILIKE $3 OR v.barcode ILIKE $3)
                ))
           AND ($4::BOOL IS NULL OR EXISTS (
                    SELECT 1 FROM product_variants v
                    WHERE v.product_id = p.id
                      AND v.is_active = $4
                ))
           AND ($5::TEXT IS NULL OR EXISTS (
                    SELECT 1 FROM product_variants v
                    WHERE v.product_id = p.id
                      AND v.is_active = TRUE
                      AND CASE $5::TEXT
                            WHEN 'out_of_stock' THEN v.stock <= 0
                            WHEN 'low' THEN (v.stock > 0 AND v.stock <= v.stock_min)
                            WHEN 'ok' THEN v.stock > v.stock_min
                            ELSE FALSE
                          END
                ))",
        is_active,
        filter.category_id,
        search,
        filter.variant_is_active,
        stock_status
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

pub async fn create_product(pool: &PgPool, dto: CreateProductDto) -> Result<Product, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let product = sqlx::query_as!(
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
    .fetch_one(&mut *tx)
    .await?;

    audit_repo::insert_event(
        &mut *tx,
        inventory_event(
            "create",
            "product",
            product.id,
            format!("Producto #{} creado: {}", product.id, product.name),
            None,
            Some(json!({
                "category_id": product.category_id,
                "name": product.name,
                "brand": product.brand,
            })),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(product)
}

pub async fn update_product(
    pool: &PgPool,
    id: i32,
    dto: UpdateProductDto,
) -> Result<Option<Product>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let before = sqlx::query_as!(
        Product,
        "SELECT * FROM products WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(before) = before else {
        return Ok(None);
    };

    let after = sqlx::query_as!(
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
    .fetch_one(&mut *tx)
    .await?;

    let changes = changed_fields(&before, &after, &AUDIT_IGNORED_FIELDS);
    if !changes.is_empty() {
        let summary = format!(
            "Producto #{} actualizado: {}",
            after.id,
            field_list(&changes)
        );
        audit_repo::insert_event(
            &mut *tx,
            inventory_event(
                "update",
                "product",
                after.id,
                summary,
                Some(JsonValue::Object(changes)),
                None,
            ),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(Some(after))
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

pub async fn get_label_data(
    pool: &PgPool,
    variant_id: i32,
) -> Result<Option<LabelData>, sqlx::Error> {
    sqlx::query_as!(
        LabelData,
        r#"SELECT
               p.name AS product_name,
               v.attributes,
               v.barcode,
               v.price
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           WHERE v.id = $1"#,
        variant_id
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
    let mut tx = pool.begin().await?;

    let cost = dto.cost.unwrap_or(0);
    let initial_stock = dto.stock.unwrap_or(0);

    let mut variant = sqlx::query_as!(
        ProductVariant,
        "INSERT INTO product_variants
             (product_id, attributes, sku, barcode, price, cost, stock, stock_min, allow_negative)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8)
         RETURNING *",
        dto.product_id,
        dto.attributes,
        dto.sku,
        dto.barcode,
        dto.price,
        cost,
        dto.stock_min.unwrap_or(0),
        dto.allow_negative.unwrap_or(false)
    )
    .fetch_one(&mut *tx)
    .await?;

    if initial_stock > 0 {
        variant = sqlx::query_as!(
            ProductVariant,
            "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
            initial_stock,
            variant.id
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO inventory_movements
                 (variant_id, movement_type, quantity, stock_before, stock_after,
                  unit_cost, notes, created_by)
             VALUES ($1, 'initial_stock', $2, 0, $2, $3, $4, 'system')",
            variant.id,
            initial_stock,
            cost,
            "Stock inicial al crear la variante"
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_repo::insert_event(
        &mut *tx,
        inventory_event(
            "create",
            "variant",
            variant.id,
            format!(
                "Variante #{} creada (producto #{})",
                variant.id, variant.product_id
            ),
            None,
            Some(json!({
                "product_id": variant.product_id,
                "attributes": variant.attributes,
                "sku": variant.sku,
                "barcode": variant.barcode,
                "price": variant.price,
                "cost": variant.cost,
                "stock": variant.stock,
                "stock_min": variant.stock_min,
                "allow_negative": variant.allow_negative,
            })),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(variant)
}

/// Campos de la variante que se auditan: los que `update_variant` puede
/// modificar. `stock` no entra (su historial es `inventory_movements`) ni
/// los timestamps. Devuelve solo los campos que cambiaron, como
/// `{"campo": {"from": .., "to": ..}}`; vacío si no cambió nada.
fn variant_changes(before: &ProductVariant, after: &ProductVariant) -> Map<String, JsonValue> {
    let mut changes = Map::new();
    let mut track = |field: &str, from: JsonValue, to: JsonValue| {
        if from != to {
            changes.insert(field.to_string(), json!({ "from": from, "to": to }));
        }
    };

    track(
        "attributes",
        before.attributes.clone(),
        after.attributes.clone(),
    );
    track("sku", json!(before.sku), json!(after.sku));
    track("barcode", json!(before.barcode), json!(after.barcode));
    track("price", json!(before.price), json!(after.price));
    track("cost", json!(before.cost), json!(after.cost));
    track("stock_min", json!(before.stock_min), json!(after.stock_min));
    track(
        "allow_negative",
        json!(before.allow_negative),
        json!(after.allow_negative),
    );
    track("is_active", json!(before.is_active), json!(after.is_active));

    changes
}

/// Actualiza la variante. Lee la fila con lock, aplica el UPDATE, compara
/// antes/después y, si algo cambió, registra el evento de auditoría en la
/// misma transacción. Si el evento falla, el cambio no se confirma.
pub async fn update_variant(
    pool: &PgPool,
    id: i32,
    dto: UpdateVariantDto,
) -> Result<Option<ProductVariant>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let before = sqlx::query_as!(
        ProductVariant,
        "SELECT * FROM product_variants WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(before) = before else {
        return Ok(None);
    };

    let after = sqlx::query_as!(
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
    .fetch_one(&mut *tx)
    .await?;

    let changes = variant_changes(&before, &after);
    if !changes.is_empty() {
        let fields = changes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");

        audit_repo::insert_event(
            &mut *tx,
            NewAuditEvent {
                correlation_id: Some(Uuid::new_v4()),
                category: AuditCategory::Business,
                module: AuditModule::Inventory,
                action: "update".to_string(),
                outcome: AuditOutcome::Success,
                actor: None,
                entity_type: Some("variant".to_string()),
                entity_id: Some(after.id.to_string()),
                summary: format!("Variante #{} actualizada: {}", after.id, fields),
                changes: Some(JsonValue::Object(changes)),
                metadata: Some(json!({ "product_id": after.product_id })),
                error_message: None,
            },
        )
        .await?;
    }

    tx.commit().await?;

    Ok(Some(after))
}

// ============================================================
// STOCK — operaciones atómicas con movimiento incluido
// ============================================================

/// Aplica entrada de stock y registra movimiento usando la
/// conexión/transacción del llamador. NO abre ni hace COMMIT.
pub async fn apply_stock_entry(
    conn: &mut PgConnection,
    dto: StockEntryDto,
    movement_type: MovementType,
) -> Result<ProductVariant, sqlx::Error> {
    // 1. Leer stock actual con lock para evitar race conditions
    let stock_before = sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let stock_after = stock_before + dto.quantity;

    // 2. Actualizar stock
    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
        stock_after,
        dto.variant_id
    )
    .fetch_one(&mut *conn)
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
    .execute(&mut *conn)
    .await?;

    Ok(variant)
}

/// Aplica salida de stock usando una conexión/transacción
/// proporcionada por el llamador.
///
/// IMPORTANTE:
/// No abre ni hace COMMIT de una transacción propia.
/// Esto permite que una venta pueda hacer:
///
/// BEGIN
///   -> crear venta
///   -> descontar inventario
///   -> registrar movimiento de caja
/// COMMIT
///
/// Si algo falla, el llamador puede hacer ROLLBACK de todo.
pub async fn apply_stock_out(
    conn: &mut PgConnection,
    dto: StockOutDto,
    movement_type: MovementType,
) -> Result<ProductVariant, sqlx::Error> {
    // 1. Bloquear la variante y obtener el stock actual
    let row = sqlx::query!(
        "SELECT stock, allow_negative
         FROM product_variants
         WHERE id = $1
         FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let stock_before = row.stock;
    let stock_after = stock_before - dto.quantity;

    // 2. Validar stock negativo
    if stock_after < 0 && !row.allow_negative {
        return Err(sqlx::Error::RowNotFound);
    }

    // 3. Actualizar stock
    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants
         SET stock = $1
         WHERE id = $2
         RETURNING *",
        stock_after,
        dto.variant_id
    )
    .fetch_one(&mut *conn)
    .await?;

    // 4. Registrar movimiento vinculado a la venta si existe sale_id
    sqlx::query!(
        "INSERT INTO inventory_movements
             (variant_id, movement_type, quantity, stock_before, stock_after,
              unit_cost, sale_id, reason, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8, $9)",
        dto.variant_id,
        movement_type as MovementType,
        dto.quantity,
        stock_before,
        stock_after,
        dto.sale_id,
        dto.reason as Option<MovementReason>,
        dto.notes,
        dto.created_by.as_deref().unwrap_or("system")
    )
    .execute(&mut *conn)
    .await?;

    Ok(variant)
}

/// Ajuste por conteo físico sobre la conexión/transacción del llamador.
pub async fn apply_stock_adjustment(
    conn: &mut PgConnection,
    dto: StockAdjustmentDto,
) -> Result<ProductVariant, sqlx::Error> {
    let stock_before = sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        dto.variant_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let diff = (dto.actual_stock - stock_before).abs();

    let variant = sqlx::query_as!(
        ProductVariant,
        "UPDATE product_variants SET stock = $1 WHERE id = $2 RETURNING *",
        dto.actual_stock,
        dto.variant_id
    )
    .fetch_one(&mut *conn)
    .await?;

    // Solo registrar movimiento si hubo diferencia
    if diff > 0 {
        sqlx::query!(
            "INSERT INTO inventory_movements
                 (variant_id, movement_type, quantity, stock_before, stock_after,
                  unit_cost, reason, notes, created_by)
             VALUES ($1, 'adjustment', $2, $3, $4, 0, $5, $6, $7)",
            dto.variant_id,
            diff,
            stock_before,
            dto.actual_stock,
            dto.reason as Option<MovementReason>,
            dto.notes,
            dto.created_by.as_deref().unwrap_or("system")
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(variant)
}

/// Stock actual con lock de fila, dentro de la transacción del llamador.
pub async fn lock_variant_stock(
    conn: &mut PgConnection,
    variant_id: i32,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        variant_id
    )
    .fetch_one(&mut *conn)
    .await
}

pub async fn next_barcode_seq(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!("SELECT nextval('internal_barcode_seq') AS \"nextval!\"")
        .fetch_one(pool)
        .await
}

pub async fn barcode_exists(pool: &PgPool, barcode: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM product_variants WHERE barcode = $1) AS \"exists!\"",
        barcode
    )
    .fetch_one(pool)
    .await
}

// ============================================================
// KARDEX
// ============================================================

pub async fn get_kardex(
    pool: &PgPool,
    filter: &KardexFilterDto,
) -> Result<(Vec<MovementWithDetails>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as!(
        MovementWithDetails,
        r#"SELECT
               m.id, m.variant_id,
               m.movement_type AS "movement_type: MovementType",
               m.quantity, m.stock_before, m.stock_after,
               m.unit_cost, m.sale_id, m.purchase_id,
               m.reason AS "reason?: MovementReason",
               m.notes, m.created_by, m.created_at,
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

pub async fn get_low_stock_items(pool: &PgPool) -> Result<Vec<LowStockItemDto>, sqlx::Error> {
    sqlx::query_as!(
        LowStockItemDto,
        r#"SELECT
               v.id AS variant_id,
               p.name AS product_name,
               v.attributes,
               v.barcode,
               v.stock,
               v.stock_min,
               CASE WHEN v.stock <= 0 THEN 'out_of_stock' ELSE 'low' END AS "stock_status!"
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           WHERE v.is_active = TRUE
             AND p.is_active = TRUE
             AND v.stock <= v.stock_min
           ORDER BY v.stock ASC, p.name ASC"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_inventory_value(pool: &PgPool) -> Result<InventoryValueDto, sqlx::Error> {
    sqlx::query_as!(
        InventoryValueDto,
        r#"SELECT
               COALESCE(
                   SUM(GREATEST(v.stock, 0)::BIGINT * v.cost)
                       FILTER (WHERE v.cost > 0), 0
               )::BIGINT AS "value_at_cost!",
               COALESCE(
                   SUM(GREATEST(v.stock, 0)::BIGINT * v.price)
                       FILTER (WHERE v.cost > 0), 0
               )::BIGINT AS "potential_sale_value!",
               COUNT(*) FILTER (WHERE v.cost = 0 AND v.stock > 0)
                   AS "variants_without_cost!",
               COUNT(*) FILTER (WHERE v.stock < 0)
                   AS "negative_stock_variants!",
               COUNT(*) FILTER (WHERE v.cost > 0)
                   AS "variants_valued!"
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           WHERE v.is_active = TRUE
             AND p.is_active = TRUE"#
    )
    .fetch_one(pool)
    .await
}

pub async fn get_product_stock_stats(
    pool: &PgPool,
    product_ids: &[i32],
) -> Result<Vec<ProductStockStatsDto>, sqlx::Error> {
    sqlx::query_as!(
        ProductStockStatsDto,
        r#"SELECT
               product_id AS "product_id!",
               COUNT(*) AS "variant_count!",
               COALESCE(SUM(stock), 0)::BIGINT AS "total_stock!",
               COALESCE(MIN(stock_min), 0) AS "min_stock_min!"
           FROM product_variants
           WHERE product_id = ANY($1)
           GROUP BY product_id"#,
        product_ids
    )
    .fetch_all(pool)
    .await
}

pub async fn get_export_rows(pool: &PgPool) -> Result<Vec<ExportVariantRow>, sqlx::Error> {
    sqlx::query_as!(
        ExportVariantRow,
        r#"SELECT
               v.id AS variant_id,
               p.id AS product_id,
               p.name AS product_name,
               p.brand,
               c.name AS category_name,
               p.description,
               v.sku,
               v.barcode,
               v.price,
               v.cost,
               v.stock,
               v.stock_min,
               v.attributes,
               v.allow_negative,
               (v.is_active AND p.is_active) AS "is_active!"
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           JOIN categories c ON c.id = p.category_id
           ORDER BY c.name, p.name, v.id"#
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn variant() -> ProductVariant {
        ProductVariant {
            id: 1,
            product_id: 7,
            attributes: json!({ "talla": "M" }),
            sku: Some("A1".to_string()),
            barcode: None,
            price: 4_500_000,
            cost: 2_000_000,
            stock: 3,
            stock_min: 1,
            allow_negative: false,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn identical_variants_have_no_changes() {
        assert!(variant_changes(&variant(), &variant()).is_empty());
    }

    #[test]
    fn only_changed_fields_are_reported_with_before_and_after() {
        let before = variant();
        let mut after = variant();
        after.price = 4_800_000;
        after.is_active = false;
        after.sku = None;

        let changes = variant_changes(&before, &after);

        assert_eq!(changes.len(), 3);
        assert_eq!(
            changes["price"],
            json!({ "from": 4_500_000, "to": 4_800_000 })
        );
        assert_eq!(changes["is_active"], json!({ "from": true, "to": false }));
        assert_eq!(changes["sku"], json!({ "from": "A1", "to": null }));
    }

    #[test]
    fn attributes_change_is_detected() {
        let before = variant();
        let mut after = variant();
        after.attributes = json!({ "talla": "L" });

        let changes = variant_changes(&before, &after);

        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes["attributes"],
            json!({ "from": { "talla": "M" }, "to": { "talla": "L" } })
        );
    }

    #[test]
    fn stock_and_timestamps_are_not_audited_here() {
        let before = variant();
        let mut after = variant();
        after.stock = 99;
        after.updated_at = Utc::now() + chrono::Duration::seconds(5);

        assert!(variant_changes(&before, &after).is_empty());
    }
}

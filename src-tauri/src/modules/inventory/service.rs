use serde_json::{json, Value as JsonValue};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};

use crate::modules::inventory::{
    dto::{
        CreateCategoryDto, CreateProductDto, CreateVariantDto, InventoryValueDto, KardexFilterDto,
        LowStockItemDto, PaginatedResponse, ProductFilterDto, ProductStockStatsDto,
        StockAdjustmentDto, StockEntryDto, StockOutDto, UpdateCategoryDto, UpdateProductDto,
        UpdateVariantDto,
    },
    models::{
        Category, MovementReason, MovementType, MovementWithDetails, Product, ProductVariant,
        ProductWithCategory, VariantWithProduct,
    },
    repository,
};

use crate::errors::app_error::AppError;

// ============================================================
// CATEGORÍAS
// ============================================================

pub async fn list_categories(pool: &PgPool) -> Result<Vec<Category>, AppError> {
    repository::get_all_categories(pool)
        .await
        .map_err(AppError::from)
}

pub async fn get_category(pool: &PgPool, id: i32) -> Result<Category, AppError> {
    repository::get_category_by_id(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Categoría no encontrada"))
}

pub async fn create_category(pool: &PgPool, dto: CreateCategoryDto) -> Result<Category, AppError> {
    if dto.name.trim().is_empty() {
        return Err(AppError::validation(
            "El nombre de la categoría es obligatorio",
        ));
    }

    repository::create_category(pool, dto).await.map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::validation("Ya existe una categoría con ese nombre")
        } else {
            AppError::from(e)
        }
    })
}

pub async fn update_category(
    pool: &PgPool,
    id: i32,
    dto: UpdateCategoryDto,
) -> Result<Category, AppError> {
    repository::update_category(pool, id, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Categoría no encontrada"))
}

// ============================================================
// PRODUCTOS
// ============================================================

pub async fn list_products(
    pool: &PgPool,
    filter: ProductFilterDto,
) -> Result<PaginatedResponse<ProductWithCategory>, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);

    let (data, total) = repository::get_products(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn get_product(pool: &PgPool, id: i32) -> Result<ProductWithCategory, AppError> {
    repository::get_product_by_id(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Producto no encontrado"))
}

pub async fn create_product(pool: &PgPool, dto: CreateProductDto) -> Result<Product, AppError> {
    if dto.name.trim().is_empty() {
        return Err(AppError::validation(
            "El nombre del producto es obligatorio",
        ));
    }

    // Verificar que la categoría existe
    repository::get_category_by_id(pool, dto.category_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::validation("La categoría especificada no existe"))?;

    repository::create_product(pool, dto)
        .await
        .map_err(AppError::from)
}

pub async fn update_product(
    pool: &PgPool,
    id: i32,
    dto: UpdateProductDto,
) -> Result<Product, AppError> {
    if let Some(ref name) = dto.name {
        if name.trim().is_empty() {
            return Err(AppError::validation(
                "El nombre del producto no puede estar vacío",
            ));
        }
    }

    repository::update_product(pool, id, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Producto no encontrado"))
}

pub async fn deactivate_product(pool: &PgPool, id: i32) -> Result<Product, AppError> {
    update_product(
        pool,
        id,
        UpdateProductDto {
            category_id: None,
            name: None,
            description: None,
            brand: None,
            image_url: None,
            is_active: Some(false),
        },
    )
    .await
}

// ============================================================
// VARIANTES
// ============================================================

pub async fn list_variants(
    pool: &PgPool,
    product_id: i32,
) -> Result<Vec<ProductVariant>, AppError> {
    repository::get_variants_by_product(pool, product_id)
        .await
        .map_err(AppError::from)
}

pub async fn get_variant(pool: &PgPool, id: i32) -> Result<ProductVariant, AppError> {
    repository::get_variant_by_id(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Variante no encontrada"))
}

/// Busca por código de barras — punto de entrada del módulo de ventas
pub async fn find_by_barcode(pool: &PgPool, barcode: &str) -> Result<VariantWithProduct, AppError> {
    if barcode.trim().is_empty() {
        return Err(AppError::validation(
            "El código de barras no puede estar vacío",
        ));
    }

    repository::get_variant_by_barcode(pool, barcode)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::not_found(&format!(
                "Producto no encontrado para el código: {}",
                barcode
            ))
        })
}

/// Búsqueda por texto — para el buscador en ventas
pub async fn search_variants(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<VariantWithProduct>, AppError> {
    if query.trim().len() < 2 {
        return Err(AppError::validation(
            "La búsqueda debe tener al menos 2 caracteres",
        ));
    }

    repository::search_variants(pool, query)
        .await
        .map_err(AppError::from)
}

pub async fn create_variant(
    pool: &PgPool,
    dto: CreateVariantDto,
) -> Result<ProductVariant, AppError> {
    if dto.price < 0 {
        return Err(AppError::validation("El precio no puede ser negativo"));
    }
    if dto.stock.unwrap_or(0) < 0 {
        return Err(AppError::validation(
            "El stock inicial no puede ser negativo",
        ));
    }
    if dto.cost.unwrap_or(0) < 0 {
        return Err(AppError::validation("El costo no puede ser negativo"));
    }

    // Verificar que el producto existe
    repository::get_product_by_id(pool, dto.product_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::validation("El producto especificado no existe"))?;

    repository::create_variant(pool, dto).await.map_err(|e| {
        let msg = e.to_string();

        if msg.contains("uq_product_attributes") {
            AppError::validation("Ya existe una variante con esos atributos para este producto")
        } else if msg.contains("product_variants_barcode_key") {
            AppError::validation("El código de barras ya está registrado en otro producto")
        } else if msg.contains("product_variants_sku_key") {
            AppError::validation("El SKU ya está en uso")
        } else {
            AppError::from(e)
        }
    })
}

pub async fn update_variant(
    pool: &PgPool,
    id: i32,
    dto: UpdateVariantDto,
) -> Result<ProductVariant, AppError> {
    if let Some(price) = dto.price {
        if price < 0 {
            return Err(AppError::validation("El precio no puede ser negativo"));
        }
    }

    repository::update_variant(pool, id, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Variante no encontrada"))
}

// ============================================================
// MOVIMIENTOS DE STOCK
// ============================================================

/// Evento de stock sobre una variante: `changes = {"stock": {from, to}}`.
fn stock_event(
    action: &str,
    variant: &ProductVariant,
    stock_before: i32,
    actor: Option<String>,
    summary: String,
    metadata: JsonValue,
) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id: Some(Uuid::new_v4()),
        category: AuditCategory::Business,
        module: AuditModule::Inventory,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor,
        entity_type: Some("variant".to_string()),
        entity_id: Some(variant.id.to_string()),
        summary,
        changes: Some(json!({ "stock": { "from": stock_before, "to": variant.stock } })),
        metadata: Some(metadata),
        error_message: None,
    }
}

/// Entrada de stock y evento de auditoría en una sola transacción.
async fn record_stock_entry(
    pool: &PgPool,
    dto: StockEntryDto,
    movement_type: MovementType,
) -> Result<ProductVariant, AppError> {
    let actor = declared_actor(&dto.created_by);
    let quantity = dto.quantity;
    let unit_cost = dto.unit_cost;
    let notes = dto.notes.clone();
    let movement = movement_type.clone();

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let variant = repository::apply_stock_entry(&mut tx, dto, movement_type)
        .await
        .map_err(AppError::from)?;
    let stock_before = variant.stock - quantity;

    audit_repo::insert_event(
        &mut *tx,
        stock_event(
            "stock_in",
            &variant,
            stock_before,
            actor,
            format!(
                "Entrada de stock en variante #{}: {} → {}",
                variant.id, stock_before, variant.stock
            ),
            json!({
                "movement_type": movement,
                "quantity": quantity,
                "unit_cost": unit_cost,
                "notes": notes,
                "product_id": variant.product_id,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(variant)
}

pub async fn register_stock_entry(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    // Verificar que la variante existe
    get_variant(pool, dto.variant_id).await?;

    record_stock_entry(pool, dto, MovementType::Purchase).await
}

pub async fn register_initial_stock(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    get_variant(pool, dto.variant_id).await?;

    record_stock_entry(pool, dto, MovementType::InitialStock).await
}

pub async fn register_manual_entry(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    get_variant(pool, dto.variant_id).await?;

    record_stock_entry(pool, dto, MovementType::ManualIn).await
}

pub async fn register_manual_out(
    pool: &PgPool,
    dto: StockOutDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    validate_reason(dto.reason, &dto.notes)?;

    let variant = get_variant(pool, dto.variant_id).await?;

    // Validación anticipada antes de ir a la DB
    if variant.stock < dto.quantity && !variant.allow_negative {
        return Err(AppError::validation(&format!(
            "Stock insuficiente. Disponible: {}, solicitado: {}",
            variant.stock, dto.quantity
        )));
    }

    // ========================================================
    // TRANSACCIÓN
    // ========================================================
    //
    // apply_stock_out ahora recibe &mut PgConnection y NO
    // crea/committea su propia transacción.
    //
    // Por eso este servicio debe abrirla.
    //

    let actor = declared_actor(&dto.created_by);
    let quantity = dto.quantity;
    let reason = dto.reason;
    let notes = dto.notes.clone();

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let updated_variant = repository::apply_stock_out(&mut tx, dto, MovementType::ManualOut)
        .await
        .map_err(AppError::from)?;

    let stock_before = updated_variant.stock + quantity;

    audit_repo::insert_event(
        &mut *tx,
        stock_event(
            "stock_out",
            &updated_variant,
            stock_before,
            actor,
            format!(
                "Salida de stock en variante #{}: {} → {}",
                updated_variant.id, stock_before, updated_variant.stock
            ),
            json!({
                "movement_type": MovementType::ManualOut,
                "quantity": quantity,
                "reason": reason,
                "notes": notes,
                "product_id": updated_variant.product_id,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;

    Ok(updated_variant)
}

pub async fn adjust_stock(
    pool: &PgPool,
    dto: StockAdjustmentDto,
) -> Result<ProductVariant, AppError> {
    if dto.actual_stock < 0 {
        return Err(AppError::validation("El stock real no puede ser negativo"));
    }

    let variant = get_variant(pool, dto.variant_id).await?;

    if dto.actual_stock == variant.stock {
        return Ok(variant);
    }

    validate_reason(dto.reason, &dto.notes)?;

    if dto.actual_stock > variant.stock
        && !matches!(
            dto.reason,
            Some(MovementReason::CountCorrection | MovementReason::Other)
        )
    {
        return Err(AppError::validation(
            "El conteo es mayor al stock actual: usa «Corrección de conteo» u «Otro»",
        ));
    }

    let actor = declared_actor(&dto.created_by);
    let reason = dto.reason;
    let notes = dto.notes.clone();

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    // Stock "antes" leído dentro de la transacción, con lock.
    let stock_before = repository::lock_variant_stock(&mut tx, dto.variant_id)
        .await
        .map_err(AppError::from)?;
    let updated = repository::apply_stock_adjustment(&mut tx, dto)
        .await
        .map_err(AppError::from)?;

    if stock_before != updated.stock {
        audit_repo::insert_event(
            &mut *tx,
            stock_event(
                "stock_adjust",
                &updated,
                stock_before,
                actor,
                format!(
                    "Ajuste de stock en variante #{}: {} → {}",
                    updated.id, stock_before, updated.stock
                ),
                json!({
                    "movement_type": MovementType::Adjustment,
                    "difference": updated.stock - stock_before,
                    "reason": reason,
                    "notes": notes,
                    "product_id": updated.product_id,
                }),
            ),
        )
        .await
        .map_err(AppError::from)?;
    }

    tx.commit().await.map_err(AppError::from)?;
    Ok(updated)
}

fn validate_reason(reason: Option<MovementReason>, notes: &Option<String>) -> Result<(), AppError> {
    match reason {
        None => Err(AppError::validation(
            "Debes indicar el motivo del movimiento",
        )),
        Some(MovementReason::Other) => {
            let has_notes = notes
                .as_deref()
                .map(|note| !note.trim().is_empty())
                .unwrap_or(false);
            if has_notes {
                Ok(())
            } else {
                Err(AppError::validation(
                    "Si el motivo es «Otro», escribe una nota que lo explique",
                ))
            }
        }
        Some(_) => Ok(()),
    }
}

// ============================================================
// KARDEX
// ============================================================

pub async fn get_kardex(
    pool: &PgPool,
    filter: KardexFilterDto,
) -> Result<PaginatedResponse<MovementWithDetails>, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);

    let (data, total) = repository::get_kardex(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

// ============================================================
// ALERTAS
// ============================================================

pub async fn get_low_stock(pool: &PgPool) -> Result<Vec<LowStockItemDto>, AppError> {
    repository::get_low_stock_items(pool)
        .await
        .map_err(AppError::from)
}

pub async fn get_inventory_value(pool: &PgPool) -> Result<InventoryValueDto, AppError> {
    repository::get_inventory_value(pool)
        .await
        .map_err(AppError::from)
}

pub async fn get_product_stock_stats(
    pool: &PgPool,
    product_ids: Vec<i32>,
) -> Result<Vec<ProductStockStatsDto>, AppError> {
    if product_ids.is_empty() {
        return Ok(Vec::new());
    }

    repository::get_product_stock_stats(pool, &product_ids)
        .await
        .map_err(AppError::from)
}

pub async fn generate_internal_barcode(pool: &PgPool) -> Result<String, AppError> {
    for _ in 0..5 {
        let sequence = repository::next_barcode_seq(pool)
            .await
            .map_err(AppError::from)?;
        let barcode = super::barcode::internal_ean13(sequence)
            .ok_or_else(|| AppError::validation("Se agotó la secuencia de códigos internos"))?;
        if !repository::barcode_exists(pool, &barcode)
            .await
            .map_err(AppError::from)?
        {
            return Ok(barcode);
        }
    }
    Err(AppError::validation(
        "No se pudo generar un código único, intenta de nuevo",
    ))
}

pub async fn print_variant_labels(
    pool: &PgPool,
    variant_id: i32,
    copies: u32,
) -> Result<(), AppError> {
    let data = repository::get_label_data(pool, variant_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Variante no encontrada"))?;

    let bytes =
        super::label::build_labels(&data, copies).map_err(|error| AppError::validation(&error))?;

    tokio::task::spawn_blocking(move || crate::modules::billing::printer::print_raw(&bytes))
        .await
        .map_err(|error| AppError::internal(&format!("Falló la impresión: {}", error)))?
        .map_err(|error| AppError::internal(&error))
}

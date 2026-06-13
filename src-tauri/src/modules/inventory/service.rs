use sqlx::PgPool;

use crate::modules::inventory::{
    dto::{
        CreateCategoryDto, CreateProductDto, CreateVariantDto, KardexFilterDto,
        LowStockItemDto, PaginatedResponse, ProductFilterDto, StockAdjustmentDto,
        StockEntryDto, StockOutDto, UpdateCategoryDto, UpdateProductDto, UpdateVariantDto,
    },
    models::{
        Category, InventoryMovement, MovementType, MovementWithDetails, Product,
        ProductVariant, ProductWithCategory, StockStatus, VariantWithProduct,
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

pub async fn create_category(
    pool: &PgPool,
    dto: CreateCategoryDto,
) -> Result<Category, AppError> {
    if dto.name.trim().is_empty() {
        return Err(AppError::validation("El nombre de la categoría es obligatorio"));
    }

    repository::create_category(pool, dto)
        .await
        .map_err(|e| {
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
    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20);

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

pub async fn create_product(
    pool: &PgPool,
    dto: CreateProductDto,
) -> Result<Product, AppError> {
    if dto.name.trim().is_empty() {
        return Err(AppError::validation("El nombre del producto es obligatorio"));
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
            return Err(AppError::validation("El nombre del producto no puede estar vacío"));
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
            name:        None,
            description: None,
            brand:       None,
            image_url:   None,
            is_active:   Some(false),
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
pub async fn find_by_barcode(
    pool: &PgPool,
    barcode: &str,
) -> Result<VariantWithProduct, AppError> {
    if barcode.trim().is_empty() {
        return Err(AppError::validation("El código de barras no puede estar vacío"));
    }

    repository::get_variant_by_barcode(pool, barcode)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found(
            &format!("Producto no encontrado para el código: {}", barcode)
        ))
}

/// Búsqueda por texto — para el buscador en ventas
pub async fn search_variants(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<VariantWithProduct>, AppError> {
    if query.trim().len() < 2 {
        return Err(AppError::validation("La búsqueda debe tener al menos 2 caracteres"));
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

pub async fn register_stock_entry(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    // Verificar que la variante existe
    get_variant(pool, dto.variant_id).await?;

    repository::apply_stock_entry(pool, dto, MovementType::Purchase)
        .await
        .map_err(AppError::from)
}

pub async fn register_initial_stock(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    get_variant(pool, dto.variant_id).await?;

    repository::apply_stock_entry(pool, dto, MovementType::InitialStock)
        .await
        .map_err(AppError::from)
}

pub async fn register_manual_entry(
    pool: &PgPool,
    dto: StockEntryDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    get_variant(pool, dto.variant_id).await?;

    repository::apply_stock_entry(pool, dto, MovementType::ManualIn)
        .await
        .map_err(AppError::from)
}

pub async fn register_manual_out(
    pool: &PgPool,
    dto: StockOutDto,
) -> Result<ProductVariant, AppError> {
    if dto.quantity <= 0 {
        return Err(AppError::validation("La cantidad debe ser mayor a cero"));
    }

    let variant = get_variant(pool, dto.variant_id).await?;

    // Validación anticipada antes de ir a la DB
    if variant.stock < dto.quantity && !variant.allow_negative {
        return Err(AppError::validation(&format!(
            "Stock insuficiente. Disponible: {}, solicitado: {}",
            variant.stock, dto.quantity
        )));
    }

    repository::apply_stock_out(pool, dto, MovementType::ManualOut)
        .await
        .map_err(AppError::from)
}

pub async fn adjust_stock(
    pool: &PgPool,
    dto: StockAdjustmentDto,
) -> Result<ProductVariant, AppError> {
    if dto.actual_stock < 0 {
        return Err(AppError::validation("El stock real no puede ser negativo"));
    }

    get_variant(pool, dto.variant_id).await?;

    repository::apply_stock_adjustment(pool, dto)
        .await
        .map_err(AppError::from)
}

// ============================================================
// KARDEX
// ============================================================

pub async fn get_kardex(
    pool: &PgPool,
    filter: KardexFilterDto,
) -> Result<PaginatedResponse<MovementWithDetails>, AppError> {
    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50);

    let (data, total) = repository::get_kardex(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

// ============================================================
// ALERTAS
// ============================================================

pub async fn get_low_stock(pool: &PgPool) -> Result<Vec<LowStockItemDto>, AppError> {
    let variants = repository::get_low_stock_variants(pool)
        .await
        .map_err(AppError::from)?;

    // Necesitamos el nombre del producto — hacemos query separado por ahora
    // (se puede optimizar con join en el repo si el volumen lo requiere)
    let mut result = Vec::new();
    for v in variants {
        let product = repository::get_product_by_id(pool, v.product_id)
            .await
            .map_err(AppError::from)?;

        let product_name = product
            .map(|p| p.name)
            .unwrap_or_else(|| "Producto eliminado".to_string());

        let stock_status = match v.stock_status() {
            StockStatus::OutOfStock => "out_of_stock",
            StockStatus::Low        => "low",
            StockStatus::Ok         => "ok",
        }
        .to_string();

        result.push(LowStockItemDto {
            variant_id: v.id,
            product_name,
            attributes: v.attributes,
            barcode: v.barcode,
            stock: v.stock,
            stock_min: v.stock_min,
            stock_status,
        });
    }

    Ok(result)
}
use tauri::State;

use crate::db::DbPool;
use crate::modules::inventory::{
    dto::{
        CreateCategoryDto, CreateProductDto, CreateVariantDto, KardexFilterDto,
        LowStockItemDto, PaginatedResponse, ProductFilterDto, StockAdjustmentDto,
        StockEntryDto, StockOutDto, UpdateCategoryDto, UpdateProductDto, UpdateVariantDto,
    },
    models::{Category, Product, ProductVariant, ProductWithCategory, VariantWithProduct, MovementWithDetails},
    service,
};
use crate::errors::app_error::AppError;

// ============================================================
// CATEGORÍAS
// ============================================================

#[tauri::command]
pub async fn list_categories(
    pool: State<'_, DbPool>,
) -> Result<Vec<Category>, String> {
    service::list_categories(&pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_category(
    pool: State<'_, DbPool>,
    id: i32,
) -> Result<Category, String> {
    service::get_category(&pool, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_category(
    pool: State<'_, DbPool>,
    dto: CreateCategoryDto,
) -> Result<Category, String> {
    service::create_category(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_category(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateCategoryDto,
) -> Result<Category, String> {
    service::update_category(&pool, id, dto)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================
// PRODUCTOS
// ============================================================

#[tauri::command]
pub async fn list_products(
    pool: State<'_, DbPool>,
    filter: ProductFilterDto,
) -> Result<PaginatedResponse<ProductWithCategory>, String> {
    service::list_products(&pool, filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product(
    pool: State<'_, DbPool>,
    id: i32,
) -> Result<ProductWithCategory, String> {
    service::get_product(&pool, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_product(
    pool: State<'_, DbPool>,
    dto: CreateProductDto,
) -> Result<Product, String> {
    service::create_product(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateProductDto,
) -> Result<Product, String> {
    service::update_product(&pool, id, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn deactivate_product(
    pool: State<'_, DbPool>,
    id: i32,
) -> Result<Product, String> {
    service::deactivate_product(&pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================
// VARIANTES
// ============================================================

#[tauri::command]
pub async fn list_variants(
    pool: State<'_, DbPool>,
    product_id: i32,
) -> Result<Vec<ProductVariant>, String> {
    service::list_variants(&pool, product_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_variant(
    pool: State<'_, DbPool>,
    id: i32,
) -> Result<ProductVariant, String> {
    service::get_variant(&pool, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_by_barcode(
    pool: State<'_, DbPool>,
    barcode: String,
) -> Result<VariantWithProduct, String> {
    service::find_by_barcode(&pool, &barcode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_variants(
    pool: State<'_, DbPool>,
    query: String,
) -> Result<Vec<VariantWithProduct>, String> {
    service::search_variants(&pool, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_variant(
    pool: State<'_, DbPool>,
    dto: CreateVariantDto,
) -> Result<ProductVariant, String> {
    service::create_variant(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_variant(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateVariantDto,
) -> Result<ProductVariant, String> {
    service::update_variant(&pool, id, dto)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================
// MOVIMIENTOS DE STOCK
// ============================================================

#[tauri::command]
pub async fn register_stock_entry(
    pool: State<'_, DbPool>,
    dto: StockEntryDto,
) -> Result<ProductVariant, String> {
    service::register_stock_entry(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_initial_stock(
    pool: State<'_, DbPool>,
    dto: StockEntryDto,
) -> Result<ProductVariant, String> {
    service::register_initial_stock(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_manual_entry(
    pool: State<'_, DbPool>,
    dto: StockEntryDto,
) -> Result<ProductVariant, String> {
    service::register_manual_entry(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_manual_out(
    pool: State<'_, DbPool>,
    dto: StockOutDto,
) -> Result<ProductVariant, String> {
    service::register_manual_out(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_stock(
    pool: State<'_, DbPool>,
    dto: StockAdjustmentDto,
) -> Result<ProductVariant, String> {
    service::adjust_stock(&pool, dto)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================
// KARDEX
// ============================================================

#[tauri::command]
pub async fn get_kardex(
    pool: State<'_, DbPool>,
    filter: KardexFilterDto,
) -> Result<PaginatedResponse<MovementWithDetails>, String> {
    service::get_kardex(&pool, filter)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================
// ALERTAS
// ============================================================

#[tauri::command]
pub async fn get_low_stock(
    pool: State<'_, DbPool>,
) -> Result<Vec<LowStockItemDto>, String> {
    service::get_low_stock(&pool)
        .await
        .map_err(|e| e.to_string())
}
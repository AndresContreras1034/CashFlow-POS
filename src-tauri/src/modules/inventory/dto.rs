use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::modules::inventory::models::{MovementReason, StockStatus};

// ============================================================
// CATEGORÍAS
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateCategoryDto {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================================
// PRODUCTOS
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateProductDto {
    pub category_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductDto {
    pub category_id: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub image_url: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================================
// VARIANTES
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateVariantDto {
    pub product_id: i32,
    pub attributes: JsonValue,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price: i64,
    pub cost: Option<i64>,
    pub stock: Option<i32>,
    pub stock_min: Option<i32>,
    pub allow_negative: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVariantDto {
    pub attributes: Option<JsonValue>,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price: Option<i64>,
    pub cost: Option<i64>,
    pub stock_min: Option<i32>,
    pub allow_negative: Option<bool>,
    pub is_active: Option<bool>,
}

// ============================================================
// MOVIMIENTOS DE INVENTARIO
// ============================================================

#[derive(Debug, Deserialize)]
pub struct StockEntryDto {
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_cost: Option<i64>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockOutDto {
    pub variant_id: i32,
    pub quantity: i32,
    pub reason: Option<MovementReason>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub sale_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentDto {
    pub variant_id: i32,
    pub actual_stock: i32,
    pub reason: Option<MovementReason>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

// ============================================================
// FILTROS / BÚSQUEDA
// ============================================================

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductSortBy {
    Name,
    Brand,
    Category,
    CreatedAt,
}

impl ProductSortBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProductSortBy::Name => "name",
            ProductSortBy::Brand => "brand",
            ProductSortBy::Category => "category",
            ProductSortBy::CreatedAt => "created_at",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProductFilterDto {
    pub category_id: Option<i32>,
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub variant_is_active: Option<bool>,
    pub stock_status: Option<StockStatus>,
    pub sort_by: Option<ProductSortBy>,
    pub sort_dir: Option<SortDir>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct KardexFilterDto {
    pub variant_id: Option<i32>,
    pub product_id: Option<i32>,
    pub movement_type: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ============================================================
// RESPUESTAS PAGINADAS
// ============================================================

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = (total + page_size - 1) / page_size;

        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

// ============================================================
// ALERTAS DE STOCK
// ============================================================

#[derive(Debug, Serialize)]
pub struct LowStockItemDto {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: JsonValue,
    pub barcode: Option<String>,
    pub stock: i32,
    pub stock_min: i32,
    pub stock_status: String,
}

/// Valoración del inventario activo. Todos los importes están en centavos.
#[derive(Debug, Serialize)]
pub struct InventoryValueDto {
    pub value_at_cost: i64,
    pub potential_sale_value: i64,
    pub variants_without_cost: i64,
    pub negative_stock_variants: i64,
    pub variants_valued: i64,
}

#[derive(Debug, Serialize)]
pub struct ProductStockStatsDto {
    pub product_id: i32,
    pub variant_count: i64,
    pub total_stock: i64,
    pub min_stock_min: i32,
}

/// Datos mínimos para imprimir la etiqueta de una variante. Precio en centavos.
#[derive(Debug)]
pub struct LabelData {
    pub product_name: String,
    pub attributes: JsonValue,
    pub barcode: Option<String>,
    pub price: i64,
}

/// Fila de variante para exportar a Excel. Importes en centavos.
#[derive(Debug)]
pub struct ExportVariantRow {
    pub variant_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub brand: Option<String>,
    pub category_name: String,
    pub description: Option<String>,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price: i64,
    pub cost: i64,
    pub stock: i32,
    pub stock_min: i32,
    pub attributes: JsonValue,
    pub allow_negative: bool,
    /// Variante activa y producto activo.
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportSummaryDto {
    pub active_rows: usize,
    pub inactive_rows: usize,
    pub low_stock_rows: usize,
}

// ============================================================
// IMPORTACIÓN MASIVA (EXCEL)
// ============================================================

#[derive(Debug, Serialize, Clone)]
pub struct ImportRowResult {
    pub row_number: usize,
    pub product_name: String,
    pub sku: Option<String>,
    pub status: String, // "new_product" | "existing_product" | "skipped_duplicate_sku" | "error"
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportSummaryDto {
    pub new_categories: i32,
    pub new_products: i32,
    pub new_variants: i32,
    pub skipped: i32,
    pub errors: i32,
    pub rows: Vec<ImportRowResult>,
    pub total_rows: usize,
    pub rows_truncated: bool,
    pub can_execute: bool,
}

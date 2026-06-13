use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================
// CATEGORÍAS
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateCategoryDto {
    pub name:        String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryDto {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub is_active:   Option<bool>,
}

// ============================================================
// PRODUCTOS
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateProductDto {
    pub category_id: i32,
    pub name:        String,
    pub description: Option<String>,
    pub brand:       Option<String>,
    pub image_url:   Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductDto {
    pub category_id: Option<i32>,
    pub name:        Option<String>,
    pub description: Option<String>,
    pub brand:       Option<String>,
    pub image_url:   Option<String>,
    pub is_active:   Option<bool>,
}

// ============================================================
// VARIANTES
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateVariantDto {
    pub product_id:     i32,
    pub attributes:     JsonValue,
    pub sku:            Option<String>,
    pub barcode:        Option<String>,
    pub price:          i64,
    pub cost:           Option<i64>,
    pub stock:          Option<i32>,
    pub stock_min:      Option<i32>,
    pub allow_negative: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVariantDto {
    pub attributes:     Option<JsonValue>,
    pub sku:            Option<String>,
    pub barcode:        Option<String>,
    pub price:          Option<i64>,
    pub cost:           Option<i64>,
    pub stock_min:      Option<i32>,
    pub allow_negative: Option<bool>,
    pub is_active:      Option<bool>,
}

// ============================================================
// MOVIMIENTOS DE INVENTARIO
// ============================================================

#[derive(Debug, Deserialize)]
pub struct StockEntryDto {
    pub variant_id: i32,
    pub quantity:   i32,
    pub unit_cost:  Option<i64>,
    pub notes:      Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockOutDto {
    pub variant_id: i32,
    pub quantity:   i32,
    pub notes:      Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentDto {
    pub variant_id:   i32,
    pub actual_stock: i32,
    pub notes:        Option<String>,
    pub created_by:   Option<String>,
}

// ============================================================
// FILTROS / BÚSQUEDA
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ProductFilterDto {
    pub category_id: Option<i32>,
    pub search:      Option<String>,
    pub is_active:   Option<bool>,
    pub page:        Option<i64>,
    pub page_size:   Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct KardexFilterDto {
    pub variant_id:    Option<i32>,
    pub product_id:    Option<i32>,
    pub movement_type: Option<String>,
    pub date_from:     Option<NaiveDate>,
    pub date_to:       Option<NaiveDate>,
    pub page:          Option<i64>,
    pub page_size:     Option<i64>,
}

// ============================================================
// RESPUESTAS PAGINADAS
// ============================================================

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data:        Vec<T>,
    pub total:       i64,
    pub page:        i64,
    pub page_size:   i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = (total + page_size - 1) / page_size;
        Self { data, total, page, page_size, total_pages }
    }
}

// ============================================================
// ALERTAS DE STOCK
// ============================================================

#[derive(Debug, Serialize)]
pub struct LowStockItemDto {
    pub variant_id:   i32,
    pub product_name: String,
    pub attributes:   JsonValue,
    pub barcode:      Option<String>,
    pub stock:        i32,
    pub stock_min:    i32,
    pub stock_status: String,
}
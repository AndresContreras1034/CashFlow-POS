use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::models::Stocktake;

#[derive(Debug, Deserialize)]
pub struct CreateStocktakeDto {
    pub category_id: Option<i32>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StocktakeLineFilterDto {
    pub search: Option<String>,
    pub only_pending: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Conteo a ciegas: no incluye el stock esperado ni el actual.
#[derive(Debug, Serialize)]
pub struct StocktakeCountLineDto {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: JsonValue,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub counted_stock: Option<i32>,
}

/// Línea de revisión: solo variantes contadas que requieren ajuste.
#[derive(Debug, Serialize)]
pub struct StocktakeReviewLineDto {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: JsonValue,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub expected_stock: i32,
    pub current_stock: i32,
    pub counted_stock: i32,
    pub difference: i64,
    pub moved_during_count: bool,
}

#[derive(Debug, Serialize)]
pub struct StocktakeSummaryDto {
    pub uncounted: i64,
    pub matching: i64,
    pub with_difference: i64,
    pub surplus_units: i64,
    pub shortage_units: i64,
    pub moved_during_count: i64,
}

#[derive(Debug, Serialize)]
pub struct StocktakeReviewDto {
    pub stocktake: Stocktake,
    pub summary: StocktakeSummaryDto,
    pub lines: Vec<StocktakeReviewLineDto>,
}

#[derive(Debug, Serialize)]
pub struct StocktakeApplyResultDto {
    pub adjusted: usize,
    pub unchanged: usize,
    pub uncounted: usize,
}

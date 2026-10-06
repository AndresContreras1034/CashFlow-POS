use tauri::State;

use crate::db::DbPool;
use crate::modules::inventory::dto::PaginatedResponse;

use super::dto::{
    CreateStocktakeDto, StocktakeApplyResultDto, StocktakeCountLineDto, StocktakeLineFilterDto,
    StocktakeReviewDto,
};
use super::models::Stocktake;
use super::service;

#[tauri::command]
pub async fn start_stocktake(
    pool: State<'_, DbPool>,
    dto: CreateStocktakeDto,
) -> Result<Stocktake, String> {
    service::start_stocktake(&pool, dto)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_current_stocktake(pool: State<'_, DbPool>) -> Result<Option<Stocktake>, String> {
    service::get_current_stocktake(&pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_stocktakes(pool: State<'_, DbPool>) -> Result<Vec<Stocktake>, String> {
    service::list_stocktakes(&pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_stocktake_lines(
    pool: State<'_, DbPool>,
    stocktake_id: i32,
    filter: StocktakeLineFilterDto,
) -> Result<PaginatedResponse<StocktakeCountLineDto>, String> {
    service::list_count_lines(&pool, stocktake_id, filter)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn find_stocktake_line_by_code(
    pool: State<'_, DbPool>,
    stocktake_id: i32,
    code: String,
) -> Result<Vec<StocktakeCountLineDto>, String> {
    service::find_count_lines_by_code(&pool, stocktake_id, &code)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_stocktake_count(
    pool: State<'_, DbPool>,
    stocktake_id: i32,
    variant_id: i32,
    counted_stock: Option<i32>,
) -> Result<(), String> {
    service::set_count(&pool, stocktake_id, variant_id, counted_stock)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_stocktake_review(
    pool: State<'_, DbPool>,
    stocktake_id: i32,
) -> Result<StocktakeReviewDto, String> {
    service::get_review(&pool, stocktake_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn apply_stocktake(
    pool: State<'_, DbPool>,
    stocktake_id: i32,
    created_by: Option<String>,
) -> Result<StocktakeApplyResultDto, String> {
    service::apply_stocktake(&pool, stocktake_id, created_by)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cancel_stocktake(pool: State<'_, DbPool>, stocktake_id: i32) -> Result<(), String> {
    service::cancel_stocktake(&pool, stocktake_id)
        .await
        .map_err(|error| error.to_string())
}

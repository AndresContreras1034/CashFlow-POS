use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::sales::{
    dto::{CreateSaleDto, PaginatedResponse, SaleFilterDto},
    models::{Sale, SaleDetail},
    service,
};

#[tauri::command]
pub async fn create_sale(
    pool: State<'_, DbPool>,
    dto: CreateSaleDto,
) -> Result<SaleDetail, String> {
    service::create_sale(&pool, dto)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_sale(pool: State<'_, DbPool>, id: i32) -> Result<SaleDetail, String> {
    service::get_sale(&pool, id)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_sales(
    pool: State<'_, DbPool>,
    filter: SaleFilterDto,
) -> Result<PaginatedResponse<Sale>, String> {
    service::list_sales(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

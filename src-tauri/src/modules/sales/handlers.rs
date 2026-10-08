use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
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
    let ctx = FailureContext {
        module: AuditModule::Sales,
        action: "create",
        entity_type: Some("sale"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(&pool, ctx, service::create_sale(&pool, dto).await).await
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

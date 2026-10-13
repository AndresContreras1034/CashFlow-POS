use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
use crate::modules::sales::{
    dto::{AssignSaleCustomerDto, CreateSaleDto, SaleFilterDto, SaleListResponse, VoidSaleDto},
    models::SaleDetail,
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
pub async fn assign_sale_customer(
    pool: State<'_, DbPool>,
    id: i32,
    dto: AssignSaleCustomerDto,
) -> Result<SaleDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Sales,
        action: "assign_customer",
        entity_type: Some("sale"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.performed_by),
    };
    finish(&pool, ctx, service::assign_customer(&pool, id, dto).await).await
}

#[tauri::command]
pub async fn void_sale(
    pool: State<'_, DbPool>,
    id: i32,
    dto: VoidSaleDto,
) -> Result<SaleDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Sales,
        action: "void",
        entity_type: Some("sale"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.performed_by),
    };
    finish(&pool, ctx, service::void_sale(&pool, id, dto).await).await
}

#[tauri::command]
pub async fn list_sales(
    pool: State<'_, DbPool>,
    filter: SaleFilterDto,
) -> Result<SaleListResponse, String> {
    service::list_sales(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

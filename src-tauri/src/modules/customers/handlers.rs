use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
use crate::modules::customers::{
    dto::{
        CreateCustomerDto, CustomerFilterDto, PaginatedResponse, SetCustomerActiveDto,
        UpdateCustomerDto,
    },
    models::{Customer, CustomerName, CustomerOverview},
    service,
};

#[tauri::command]
pub async fn list_customers(
    pool: State<'_, DbPool>,
    filter: CustomerFilterDto,
) -> Result<PaginatedResponse<CustomerOverview>, String> {
    service::list_customers(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_customer(pool: State<'_, DbPool>, id: i32) -> Result<CustomerOverview, String> {
    service::get_customer(&pool, id)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_customer_names(
    pool: State<'_, DbPool>,
    ids: Vec<i32>,
) -> Result<Vec<CustomerName>, String> {
    service::get_customer_names(&pool, ids)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn create_customer(
    pool: State<'_, DbPool>,
    dto: CreateCustomerDto,
) -> Result<Customer, String> {
    let context = FailureContext {
        module: AuditModule::Customers,
        action: "create",
        entity_type: Some("customer"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(&pool, context, service::create_customer(&pool, dto).await).await
}

#[tauri::command]
pub async fn update_customer(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateCustomerDto,
) -> Result<Customer, String> {
    let context = FailureContext {
        module: AuditModule::Customers,
        action: "update",
        entity_type: Some("customer"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::update_customer(&pool, id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn set_customer_active(
    pool: State<'_, DbPool>,
    id: i32,
    dto: SetCustomerActiveDto,
) -> Result<Customer, String> {
    let context = FailureContext {
        module: AuditModule::Customers,
        action: "set_active",
        entity_type: Some("customer"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::set_customer_active(&pool, id, dto).await,
    )
    .await
}

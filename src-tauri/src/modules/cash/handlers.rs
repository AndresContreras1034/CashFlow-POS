use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
use crate::modules::cash::{
    dto::{
        CashMovementFilterDto, CashSessionFilterDto, CloseSessionDto, CreateMovementDto,
        OpenSessionDto, PaginatedResponse,
    },
    models::{CashMovement, CashSession, CashSessionWithTotals},
    service,
};

#[tauri::command]
pub async fn open_cash_session(
    pool: State<'_, DbPool>,
    dto: OpenSessionDto,
) -> Result<CashSession, String> {
    let ctx = FailureContext {
        module: AuditModule::Cash,
        action: "open",
        entity_type: Some("cash_session"),
        entity_id: None,
        actor: declared_actor(&dto.opened_by),
    };
    finish(&pool, ctx, service::open_session(&pool, dto).await).await
}

#[tauri::command]
pub async fn close_cash_session(
    pool: State<'_, DbPool>,
    id: i32,
    dto: CloseSessionDto,
) -> Result<CashSession, String> {
    let ctx = FailureContext {
        module: AuditModule::Cash,
        action: "close",
        entity_type: Some("cash_session"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.closed_by),
    };
    finish(&pool, ctx, service::close_session(&pool, id, dto).await).await
}

#[tauri::command]
pub async fn get_current_cash_session(
    pool: State<'_, DbPool>,
) -> Result<Option<CashSessionWithTotals>, String> {
    service::get_current_session(&pool)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_cash_session(pool: State<'_, DbPool>, id: i32) -> Result<CashSession, String> {
    service::get_session(&pool, id)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_cash_sessions(
    pool: State<'_, DbPool>,
    filter: CashSessionFilterDto,
) -> Result<PaginatedResponse<CashSession>, String> {
    service::list_sessions(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

// OJO: el parámetro se llama session_id (snake_case) porque Tauri exige que la
// clave top-level del objeto de argumentos venga en camelCase desde el frontend
// (invoke('register_cash_movement', { sessionId, dto })) y él mismo la traduce.
#[tauri::command]
pub async fn register_cash_movement(
    pool: State<'_, DbPool>,
    session_id: i32,
    dto: CreateMovementDto,
) -> Result<CashMovement, String> {
    let ctx = FailureContext {
        module: AuditModule::Cash,
        action: "create",
        entity_type: Some("cash_movement"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(
        &pool,
        ctx,
        service::register_manual_movement(&pool, session_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn list_cash_movements(
    pool: State<'_, DbPool>,
    filter: CashMovementFilterDto,
) -> Result<PaginatedResponse<CashMovement>, String> {
    service::list_movements(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

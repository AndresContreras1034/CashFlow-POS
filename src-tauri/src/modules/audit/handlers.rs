use tauri::State;
use uuid::Uuid;

use crate::db::DbPool;
use crate::modules::audit::{dto::AuditEventFilterDto, models::AuditEvent, service};
use crate::modules::inventory::dto::PaginatedResponse;

#[tauri::command]
pub async fn list_audit_events(
    pool: State<'_, DbPool>,
    filter: AuditEventFilterDto,
) -> Result<PaginatedResponse<AuditEvent>, String> {
    service::list_events(&pool, filter)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_audit_event(pool: State<'_, DbPool>, id: i64) -> Result<AuditEvent, String> {
    service::get_event(&pool, id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_audit_events_by_correlation(
    pool: State<'_, DbPool>,
    correlation_id: Uuid,
) -> Result<Vec<AuditEvent>, String> {
    service::list_by_correlation(&pool, correlation_id)
        .await
        .map_err(|error| error.to_string())
}

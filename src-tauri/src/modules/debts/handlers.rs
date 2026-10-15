use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::{AuditEvent, AuditModule},
};
use crate::modules::debts::{
    catalog::DebtOptions,
    dto::{
        CreateDebtDto, DebtFilterDto, DueDebtsFilterDto, PaginatedResponse, RegisterPaymentDto,
        UpdateDebtDto, VoidDebtDto, VoidPaymentDto,
    },
    models::{DebtCategoryStat, DebtDetail, DebtOverview, DebtSummary},
    service,
};

// OJO: los parámetros de primer nivel (`debt_id`, `payment_id`) van en
// snake_case en Rust, pero el frontend los manda en camelCase
// (`invoke('register_debt_payment', { debtId, dto })`); Tauri los traduce.

// ============================================================
// ESCRITURAS (los fallos técnicos se auditan con `finish`)
// ============================================================

#[tauri::command]
pub async fn create_debt(
    pool: State<'_, DbPool>,
    dto: CreateDebtDto,
) -> Result<DebtDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Debts,
        action: "create",
        entity_type: Some("debt"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(&pool, ctx, service::create_debt(&pool, dto).await).await
}

#[tauri::command]
pub async fn update_debt(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateDebtDto,
) -> Result<DebtDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Debts,
        action: "update",
        entity_type: Some("debt"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(&pool, ctx, service::update_debt(&pool, id, dto).await).await
}

#[tauri::command]
pub async fn void_debt(
    pool: State<'_, DbPool>,
    id: i32,
    dto: VoidDebtDto,
) -> Result<DebtDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Debts,
        action: "void",
        entity_type: Some("debt"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.voided_by),
    };
    finish(&pool, ctx, service::void_debt(&pool, id, dto).await).await
}

#[tauri::command]
pub async fn register_debt_payment(
    pool: State<'_, DbPool>,
    debt_id: i32,
    dto: RegisterPaymentDto,
) -> Result<DebtDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Debts,
        action: "create",
        entity_type: Some("debt_payment"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(
        &pool,
        ctx,
        service::register_payment(&pool, debt_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn void_debt_payment(
    pool: State<'_, DbPool>,
    payment_id: i32,
    dto: VoidPaymentDto,
) -> Result<DebtDetail, String> {
    let ctx = FailureContext {
        module: AuditModule::Debts,
        action: "void",
        entity_type: Some("debt_payment"),
        entity_id: Some(payment_id.to_string()),
        actor: declared_actor(&dto.voided_by),
    };
    finish(
        &pool,
        ctx,
        service::void_payment(&pool, payment_id, dto).await,
    )
    .await
}

// ============================================================
// LECTURAS
// ============================================================

#[tauri::command]
pub async fn get_debt(pool: State<'_, DbPool>, id: i32) -> Result<DebtDetail, String> {
    service::get_debt(&pool, id)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_debts(
    pool: State<'_, DbPool>,
    filter: DebtFilterDto,
) -> Result<PaginatedResponse<DebtOverview>, String> {
    service::list_debts(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_debt_summary(
    pool: State<'_, DbPool>,
    filter: DebtFilterDto,
) -> Result<DebtSummary, String> {
    service::get_summary(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_debt_category_stats(
    pool: State<'_, DbPool>,
    filter: DebtFilterDto,
) -> Result<Vec<DebtCategoryStat>, String> {
    service::get_category_stats(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_due_debts(
    pool: State<'_, DbPool>,
    filter: DueDebtsFilterDto,
) -> Result<PaginatedResponse<DebtOverview>, String> {
    service::list_due_debts(&pool, filter)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_debt_options() -> Result<DebtOptions, String> {
    Ok(service::get_options())
}

#[tauri::command]
pub async fn list_debt_history(
    pool: State<'_, DbPool>,
    debt_id: i32,
) -> Result<Vec<AuditEvent>, String> {
    service::list_history(&pool, debt_id)
        .await
        .map_err(|e: AppError| e.to_string())
}

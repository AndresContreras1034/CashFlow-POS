use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::cash::{
    dto::{
        CashMovementFilterDto, CashSessionFilterDto, CloseSessionDto, CreateMovementDto,
        OpenSessionDto, PaginatedResponse,
    },
    models::{CashMovement, CashMovementType, CashSession, CashSessionWithTotals},
    repository::{self, CloseSessionError},
};

// ============================================================
// SESIONES
// ============================================================

pub async fn get_current_session(pool: &PgPool) -> Result<Option<CashSessionWithTotals>, AppError> {
    repository::get_open_session_with_totals(pool)
        .await
        .map_err(AppError::from)
}

pub async fn get_session(pool: &PgPool, id: i32) -> Result<CashSession, AppError> {
    repository::get_session_by_id(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Turno de caja no encontrado"))
}

pub async fn list_sessions(
    pool: &PgPool,
    filter: CashSessionFilterDto,
) -> Result<PaginatedResponse<CashSession>, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20);

    let (data, total) = repository::list_sessions(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn open_session(pool: &PgPool, dto: OpenSessionDto) -> Result<CashSession, AppError> {
    if dto.opening_amount < 0 {
        return Err(AppError::validation(
            "El monto de apertura no puede ser negativo",
        ));
    }

    repository::open_session(pool, dto).await.map_err(|e| {
        if e.to_string().contains("uq_one_open_session") {
            AppError::validation(
                "Ya hay un turno de caja abierto. Ciérralo antes de abrir uno nuevo.",
            )
        } else {
            AppError::from(e)
        }
    })
}

pub async fn close_session(
    pool: &PgPool,
    id: i32,
    dto: CloseSessionDto,
) -> Result<CashSession, AppError> {
    if dto.counted_amount < 0 {
        return Err(AppError::validation(
            "El monto contado no puede ser negativo",
        ));
    }

    let session = get_session(pool, id).await?;

    if !session.is_open() {
        return Err(AppError::validation("Este turno ya está cerrado"));
    }

    repository::close_session(pool, id, dto)
        .await
        .map_err(|e| match e {
            CloseSessionError::Db(err) => AppError::from(err),
            CloseSessionError::AlreadyClosed => AppError::validation("Este turno ya está cerrado"),
        })?
        .ok_or_else(|| AppError::not_found("Turno de caja no encontrado"))
}

// ============================================================
// MOVIMIENTOS
// ============================================================

/// Registra un movimiento manual de caja.
///
/// Esta función abre su propia transacción porque los movimientos
/// manuales se ejecutan de forma independiente.
///
/// En una venta, en cambio, `repository::create_movement` será
/// llamado directamente usando la transacción general de la venta.
pub async fn register_manual_movement(
    pool: &PgPool,
    session_id: i32,
    dto: CreateMovementDto,
) -> Result<CashMovement, AppError> {
    // Solo permitimos movimientos manuales desde este comando.
    if !dto.movement_type.is_manual() {
        return Err(AppError::validation(
            "Solo se pueden registrar movimientos manuales de ingreso o egreso desde este comando",
        ));
    }

    // El monto debe ser positivo.
    if dto.amount <= 0 {
        return Err(AppError::validation("El monto debe ser mayor a cero"));
    }

    // `dto` se consume en create_movement: el actor se toma antes.
    let actor = declared_actor(&dto.created_by);

    // Abrimos la transacción aquí.
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    // El repository utiliza la transacción existente.
    let movement = repository::create_movement(&mut tx, session_id, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::validation("No hay un turno de caja abierto para registrar el movimiento")
        })?;

    let label = if movement.movement_type == CashMovementType::ManualIn {
        "Ingreso manual"
    } else {
        "Egreso manual"
    };

    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Cash,
            action: "create".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("cash_movement".to_string()),
            entity_id: Some(movement.id.to_string()),
            summary: format!("{} de caja en el turno #{}", label, movement.session_id),
            changes: None,
            metadata: Some(json!({
                "movement_type": movement.movement_type,
                "amount": movement.amount,
                "session_id": movement.session_id,
                "notes": movement.notes,
            })),
            error_message: None,
        },
    )
    .await
    .map_err(AppError::from)?;

    // Si todo salió bien, confirmamos.
    tx.commit().await.map_err(AppError::from)?;

    Ok(movement)
}

pub async fn list_movements(
    pool: &PgPool,
    filter: CashMovementFilterDto,
) -> Result<PaginatedResponse<CashMovement>, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50);

    let (data, total) = repository::list_movements(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

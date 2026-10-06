use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::cash::{
    dto::{
        CashMovementFilterDto, CashSessionFilterDto, CloseSessionDto, CreateMovementDto,
        OpenSessionDto, PaginatedResponse,
    },
    models::{CashMovement, CashSession, CashSessionWithTotals},
    repository,
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
        .map_err(AppError::from)?
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

    // Abrimos la transacción aquí.
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    // El repository utiliza la transacción existente.
    let movement = repository::create_movement(&mut tx, session_id, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::validation("No hay un turno de caja abierto para registrar el movimiento")
        })?;

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

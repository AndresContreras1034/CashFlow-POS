use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};

use crate::modules::cash::{
    dto::{
        CashMovementFilterDto, CashSessionFilterDto, CloseSessionDto, CreateMovementDto,
        OpenSessionDto,
    },
    models::{CashMovement, CashMovementType, CashSession, CashSessionWithTotals},
};

/// Errores de negocio propios del cierre de turno — se distinguen de
/// sqlx::Error para que el service pueda dar un mensaje claro.
pub enum CloseSessionError {
    Db(sqlx::Error),
    /// El turno ya estaba cerrado cuando se obtuvo el lock de la fila.
    AlreadyClosed,
}

impl From<sqlx::Error> for CloseSessionError {
    fn from(e: sqlx::Error) -> Self {
        CloseSessionError::Db(e)
    }
}

// ============================================================
// SESIONES DE CAJA
// ============================================================

pub async fn get_open_session(pool: &PgPool) -> Result<Option<CashSession>, sqlx::Error> {
    sqlx::query_as!(
        CashSession,
        "SELECT * FROM cash_sessions WHERE status = 'open'"
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_open_session_with_totals(
    pool: &PgPool,
) -> Result<Option<CashSessionWithTotals>, sqlx::Error> {
    sqlx::query_as!(
        CashSessionWithTotals,
        r#"SELECT
               s.id, s.opening_amount, s.status, s.opened_by,
               s.opening_notes, s.opened_at,
               COALESCE(SUM(m.amount) FILTER (
                   WHERE m.movement_type IN ('manual_in', 'sale_in')
               ), 0)::BIGINT AS "total_in!",
               COALESCE(SUM(m.amount) FILTER (
                   WHERE m.movement_type IN ('manual_out', 'sale_out')
               ), 0)::BIGINT AS "total_out!",
               (s.opening_amount
                   + COALESCE(SUM(m.amount) FILTER (
                       WHERE m.movement_type IN ('manual_in', 'sale_in')
                     ), 0)
                   - COALESCE(SUM(m.amount) FILTER (
                       WHERE m.movement_type IN ('manual_out', 'sale_out')
                     ), 0))::BIGINT AS "current_balance!"
           FROM cash_sessions s
           LEFT JOIN cash_movements m ON m.session_id = s.id
           WHERE s.status = 'open'
           GROUP BY s.id"#
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_session_by_id(pool: &PgPool, id: i32) -> Result<Option<CashSession>, sqlx::Error> {
    sqlx::query_as!(CashSession, "SELECT * FROM cash_sessions WHERE id = $1", id)
        .fetch_optional(pool)
        .await
}

pub async fn list_sessions(
    pool: &PgPool,
    filter: &CashSessionFilterDto,
) -> Result<(Vec<CashSession>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as!(
        CashSession,
        r#"SELECT * FROM cash_sessions
           WHERE ($1::TEXT IS NULL OR status = $1)
             AND ($2::DATE IS NULL OR opened_at::DATE >= $2)
             AND ($3::DATE IS NULL OR opened_at::DATE <= $3)
           ORDER BY opened_at DESC
           LIMIT $4 OFFSET $5"#,
        filter.status,
        filter.date_from,
        filter.date_to,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM cash_sessions
           WHERE ($1::TEXT IS NULL OR status = $1)
             AND ($2::DATE IS NULL OR opened_at::DATE >= $2)
             AND ($3::DATE IS NULL OR opened_at::DATE <= $3)"#,
        filter.status,
        filter.date_from,
        filter.date_to
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}
pub async fn open_session(pool: &PgPool, dto: OpenSessionDto) -> Result<CashSession, sqlx::Error> {
    let actor = declared_actor(&dto.opened_by);
    let mut tx = pool.begin().await?;

    let session = sqlx::query_as!(
        CashSession,
        "INSERT INTO cash_sessions (opening_amount, opening_notes, opened_by)
         VALUES ($1, $2, $3)
         RETURNING *",
        dto.opening_amount,
        dto.opening_notes,
        dto.opened_by.unwrap_or_else(|| "system".to_string())
    )
    .fetch_one(&mut *tx)
    .await?;

    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Cash,
            action: "open".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("cash_session".to_string()),
            entity_id: Some(session.id.to_string()),
            summary: format!("Turno de caja #{} abierto", session.id),
            changes: None,
            metadata: Some(json!({
                "opening_amount": session.opening_amount,
                "opening_notes": session.opening_notes,
            })),
            error_message: None,
        },
    )
    .await?;

    tx.commit().await?;

    Ok(session)
}

/// Cierra el turno dentro de una transacción y registra el evento de auditoría.
pub async fn close_session(
    pool: &PgPool,
    id: i32,
    dto: CloseSessionDto,
) -> Result<Option<CashSession>, CloseSessionError> {
    let actor = declared_actor(&dto.closed_by);
    let mut tx = pool.begin().await?;

    let session = sqlx::query_as!(
        CashSession,
        "SELECT * FROM cash_sessions WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(session) = session else {
        return Ok(None);
    };

    if !session.is_open() {
        return Err(CloseSessionError::AlreadyClosed);
    }

    let total_in = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(amount), 0)::BIGINT AS "total!"
           FROM cash_movements
           WHERE session_id = $1
             AND movement_type IN ('manual_in', 'sale_in')"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    let total_out = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(amount), 0)::BIGINT AS "total!"
           FROM cash_movements
           WHERE session_id = $1
             AND movement_type IN ('manual_out', 'sale_out')"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    let expected_amount = session.opening_amount + total_in - total_out;
    let difference = dto.counted_amount - expected_amount;

    let updated = sqlx::query_as!(
        CashSession,
        "UPDATE cash_sessions
         SET status          = 'closed',
             expected_amount = $1,
             counted_amount  = $2,
             difference      = $3,
             closing_notes   = $4,
             closed_by       = $5,
             closed_at       = NOW()
         WHERE id = $6
         RETURNING *",
        expected_amount,
        dto.counted_amount,
        difference,
        dto.closing_notes,
        dto.closed_by.unwrap_or_else(|| "system".to_string()),
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Cash,
            action: "close".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("cash_session".to_string()),
            entity_id: Some(updated.id.to_string()),
            summary: format!("Turno de caja #{} cerrado", updated.id),
            changes: Some(json!({
                "status": { "from": "open", "to": "closed" },
            })),
            metadata: Some(json!({
                "opening_amount": updated.opening_amount,
                "expected_amount": updated.expected_amount,
                "counted_amount": updated.counted_amount,
                "difference": updated.difference,
                "closing_notes": updated.closing_notes,
            })),
            error_message: None,
        },
    )
    .await?;

    tx.commit().await?;

    Ok(Some(updated))
}

// ============================================================
// MOVIMIENTOS
// ============================================================

/// Inserta un movimiento usando la conexión/transacción proporcionada
/// por el llamador.
///
/// NO abre una transacción propia y NO hace COMMIT.
///
/// Esto permite que una venta pueda registrar el movimiento de caja
/// dentro de la misma transacción que:
/// - la venta
/// - los items de la venta
/// - la salida de inventario
///
/// Si cualquiera de esas operaciones falla, el llamador puede hacer
/// ROLLBACK de todo.
pub async fn create_movement(
    conn: &mut PgConnection,
    session_id: i32,
    dto: CreateMovementDto,
) -> Result<Option<CashMovement>, sqlx::Error> {
    // 1. Bloquear la sesión para evitar que un movimiento
    //    se registre mientras otro proceso la está cerrando.
    let status = sqlx::query_scalar!(
        "SELECT status
         FROM cash_sessions
         WHERE id = $1
         FOR UPDATE",
        session_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    // 2. Solo se permiten movimientos en sesiones abiertas.
    match status {
        Some(s) if s == "open" => {}
        _ => return Ok(None),
    }

    // 3. Registrar el movimiento dentro de la transacción
    //    que recibió esta función.
    let movement = sqlx::query_as!(
        CashMovement,
        r#"INSERT INTO cash_movements
               (session_id, movement_type, amount, sale_id, notes, created_by)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING
               id,
               session_id,
               movement_type AS "movement_type: CashMovementType",
               amount,
               sale_id,
               notes,
               created_by,
               created_at"#,
        session_id,
        dto.movement_type as CashMovementType,
        dto.amount,
        dto.sale_id,
        dto.notes,
        dto.created_by.unwrap_or_else(|| "system".to_string())
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(Some(movement))
}

pub async fn list_movements(
    pool: &PgPool,
    filter: &CashMovementFilterDto,
) -> Result<(Vec<CashMovement>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as!(
        CashMovement,
        r#"SELECT
               id,
               session_id,
               movement_type AS "movement_type: CashMovementType",
               amount,
               sale_id,
               notes,
               created_by,
               created_at
           FROM cash_movements
           WHERE ($1::INT  IS NULL OR session_id = $1)
             AND ($2::TEXT IS NULL OR movement_type::TEXT = $2)
           ORDER BY created_at DESC
           LIMIT $3 OFFSET $4"#,
        filter.session_id,
        filter.movement_type,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*)
           FROM cash_movements
           WHERE ($1::INT  IS NULL OR session_id = $1)
             AND ($2::TEXT IS NULL OR movement_type::TEXT = $2)"#,
        filter.session_id,
        filter.movement_type
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

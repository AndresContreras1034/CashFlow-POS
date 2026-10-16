use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    errors::app_error::AppError,
    modules::{
        audit::{
            dto::NewAuditEvent,
            models::{AuditCategory, AuditModule, AuditOutcome},
            repository as audit_repository,
        },
        commercial_calendar::dto::{CommercialEvent, CommercialEventInput},
    },
};

const SELECT_EVENT: &str = r#"
    SELECT id, name, kind, rule, event_date, month, day, weekday, nth, prep_days, is_active
    FROM commercial_events
"#;

pub async fn list_events(pool: &PgPool) -> Result<Vec<CommercialEvent>, sqlx::Error> {
    sqlx::query_as::<_, CommercialEvent>(&format!(
        "{SELECT_EVENT} ORDER BY is_active DESC, name ASC, id ASC"
    ))
    .fetch_all(pool)
    .await
}

pub async fn create_event(
    pool: &PgPool,
    input: CommercialEventInput,
) -> Result<CommercialEvent, AppError> {
    let mut tx = pool.begin().await?;
    let event = insert_event(&mut tx, &input).await?;
    audit_repository::insert_event(
        &mut tx,
        audit_event(
            "create_commercial_event",
            Some(event.id),
            format!("Evento comercial creado: {}", event.name),
            None,
            Some(serde_json::json!({
                "name": event.name,
                "kind": event.kind,
                "rule": event.rule,
                "event_date": event.event_date,
                "month": event.month,
                "day": event.day,
                "weekday": event.weekday,
                "nth": event.nth,
                "prep_days": event.prep_days,
                "is_active": event.is_active
            })),
            input.actor,
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(event)
}

pub async fn update_event(
    pool: &PgPool,
    id: i64,
    input: CommercialEventInput,
) -> Result<CommercialEvent, AppError> {
    let mut tx = pool.begin().await?;
    let before = find_event(&mut tx, id)
        .await?
        .ok_or_else(|| AppError::not_found("Evento comercial no encontrado"))?;
    let event = sqlx::query_as::<_, CommercialEvent>(
        r#"UPDATE commercial_events
           SET name = $2, kind = $3, rule = $4, event_date = $5, month = $6,
               day = $7, weekday = $8, nth = $9, prep_days = $10, is_active = $11
           WHERE id = $1
           RETURNING id, name, kind, rule, event_date, month, day, weekday, nth, prep_days, is_active"#,
    )
    .bind(id)
    .bind(input.name)
    .bind(input.kind)
    .bind(input.rule)
    .bind(input.event_date)
    .bind(input.month)
    .bind(input.day)
    .bind(input.weekday)
    .bind(input.nth)
    .bind(input.prep_days)
    .bind(input.is_active)
    .fetch_one(&mut *tx)
    .await?;

    audit_repository::insert_event(
        &mut tx,
        audit_event(
            "update_commercial_event",
            Some(id),
            format!("Evento comercial actualizado: {}", event.name),
            Some(event_json(&before)),
            Some(event_json(&event)),
            input.actor,
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(event)
}

pub async fn delete_event(pool: &PgPool, id: i64, actor: Option<String>) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let event = find_event(&mut tx, id)
        .await?
        .ok_or_else(|| AppError::not_found("Evento comercial no encontrado"))?;
    sqlx::query("DELETE FROM commercial_events WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit_repository::insert_event(
        &mut tx,
        audit_event(
            "delete_commercial_event",
            Some(id),
            format!("Evento comercial eliminado: {}", event.name),
            Some(event_json(&event)),
            None,
            actor,
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    input: &CommercialEventInput,
) -> Result<CommercialEvent, AppError> {
    sqlx::query_as::<_, CommercialEvent>(
        r#"INSERT INTO commercial_events
               (name, kind, rule, event_date, month, day, weekday, nth, prep_days, is_active)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, kind, rule, event_date, month, day, weekday, nth, prep_days, is_active"#,
    )
    .bind(&input.name)
    .bind(&input.kind)
    .bind(&input.rule)
    .bind(input.event_date)
    .bind(input.month)
    .bind(input.day)
    .bind(input.weekday)
    .bind(input.nth)
    .bind(input.prep_days)
    .bind(input.is_active)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::from)
}

async fn find_event(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> Result<Option<CommercialEvent>, AppError> {
    sqlx::query_as::<_, CommercialEvent>(&format!("{SELECT_EVENT} WHERE id = $1 FOR UPDATE"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AppError::from)
}

pub async fn daily_sales_and_open_days(
    pool: &PgPool,
    from: NaiveDate,
    to: NaiveDate,
    timezone: &str,
) -> Result<HashMap<NaiveDate, i64>, AppError> {
    let sales = sqlx::query_as::<_, (NaiveDate, i64)>(
        r#"SELECT (created_at AT TIME ZONE $3)::DATE AS local_day,
                  COALESCE(SUM(total), 0)::BIGINT AS sales_cents
           FROM sales
           WHERE status = 'completed'
             AND courtesy_reason IS NULL
             AND created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
             AND created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
           GROUP BY local_day"#,
    )
    .bind(from)
    .bind(to)
    .bind(timezone)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    Ok(sales.into_iter().collect())
}

pub async fn earliest_paid_sale_date(
    pool: &PgPool,
    timezone: &str,
) -> Result<Option<NaiveDate>, AppError> {
    sqlx::query_scalar(
        r#"SELECT MIN((created_at AT TIME ZONE $1)::DATE)
           FROM sales
           WHERE status = 'completed' AND courtesy_reason IS NULL"#,
    )
    .bind(timezone)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

fn event_json(event: &CommercialEvent) -> serde_json::Value {
    serde_json::json!({
        "id": event.id,
        "name": event.name,
        "kind": event.kind,
        "rule": event.rule,
        "event_date": event.event_date,
        "month": event.month,
        "day": event.day,
        "weekday": event.weekday,
        "nth": event.nth,
        "prep_days": event.prep_days,
        "is_active": event.is_active
    })
}

fn audit_event(
    action: &str,
    id: Option<i64>,
    summary: String,
    changes: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
    actor: Option<String>,
) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id: None,
        category: AuditCategory::Business,
        module: AuditModule::Settings,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor,
        entity_type: Some("commercial_event".to_string()),
        entity_id: id.map(|value| value.to_string()),
        summary,
        changes,
        metadata,
        error_message: None,
    }
}

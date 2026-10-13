use serde_json::{json, Value as JsonValue};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::customers::{
    dto::{CreateCustomerDto, CustomerFilterDto, SetCustomerActiveDto, UpdateCustomerDto},
    models::{Customer, CustomerName, CustomerOverview},
};

pub async fn names_by_ids(pool: &PgPool, ids: &[i32]) -> Result<Vec<CustomerName>, sqlx::Error> {
    sqlx::query_as::<_, CustomerName>("SELECT id, name FROM customers WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(pool)
        .await
}

#[derive(Debug)]
pub enum CustomerError {
    Db(sqlx::Error),
    NotFound,
    DuplicateDocument,
}

impl From<sqlx::Error> for CustomerError {
    fn from(error: sqlx::Error) -> Self {
        let duplicate = error
            .as_database_error()
            .and_then(|db| db.constraint())
            .is_some_and(|name| name == "uq_customers_document");
        if duplicate {
            CustomerError::DuplicateDocument
        } else {
            CustomerError::Db(error)
        }
    }
}

const COLS: &str = "id, name, document_type, document_number, phone, email, address, notes, \
                    is_active, created_by, created_at, updated_at";
const COLS_C: &str = "c.id, c.name, c.document_type, c.document_number, c.phone, c.email, \
                      c.address, c.notes, c.is_active, c.created_by, c.created_at, c.updated_at";

const OVERVIEW_FROM: &str = "FROM customers c
    LEFT JOIN LATERAL (
        SELECT COUNT(*)::BIGINT                  AS sales_count,
               COALESCE(SUM(total), 0)::BIGINT   AS total_spent,
               MAX(created_at)                   AS last_purchase_at
        FROM sales
        WHERE customer_id = c.id AND status = 'completed'
    ) s ON TRUE";

const OVERVIEW_STATS: &str = "COALESCE(s.sales_count, 0)::BIGINT AS sales_count,
    COALESCE(s.total_spent, 0)::BIGINT AS total_spent,
    s.last_purchase_at AS last_purchase_at";

async fn record_event(
    conn: &mut sqlx::PgConnection,
    actor: Option<String>,
    action: &str,
    customer_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> Result<(), sqlx::Error> {
    audit_repo::insert_event(
        conn,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Customers,
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("customer".to_string()),
            entity_id: Some(customer_id.to_string()),
            summary,
            changes,
            metadata: Some(metadata),
            error_message: None,
        },
    )
    .await?;
    Ok(())
}

async fn lock_customer(
    conn: &mut sqlx::PgConnection,
    id: i32,
) -> Result<Option<Customer>, sqlx::Error> {
    sqlx::query_as::<_, Customer>(&format!(
        "SELECT {COLS} FROM customers WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn create_customer(
    pool: &PgPool,
    dto: CreateCustomerDto,
) -> Result<Customer, CustomerError> {
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await?;

    let customer = sqlx::query_as::<_, Customer>(&format!(
        "INSERT INTO customers
             (name, document_type, document_number, phone, email, address, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {COLS}"
    ))
    .bind(&dto.name)
    .bind(&dto.document_type)
    .bind(&dto.document_number)
    .bind(&dto.phone)
    .bind(&dto.email)
    .bind(&dto.address)
    .bind(&dto.notes)
    .bind(&created_by)
    .fetch_one(&mut *tx)
    .await?;

    record_event(
        &mut tx,
        actor,
        "create",
        customer.id,
        format!("Cliente #{} registrado: {}", customer.id, customer.name),
        None,
        json!({
            "name": customer.name,
            "document_type": customer.document_type,
            "document_number": customer.document_number,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(customer)
}

/// Si no cambia nada, no escribe ni audita.
pub async fn update_customer(
    pool: &PgPool,
    id: i32,
    dto: UpdateCustomerDto,
) -> Result<Customer, CustomerError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;

    let before = lock_customer(&mut tx, id)
        .await?
        .ok_or(CustomerError::NotFound)?;

    let after = sqlx::query_as::<_, Customer>(&format!(
        "UPDATE customers
         SET name = $2, document_type = $3, document_number = $4,
             phone = $5, email = $6, address = $7, notes = $8
         WHERE id = $1
         RETURNING {COLS}"
    ))
    .bind(id)
    .bind(&dto.name)
    .bind(&dto.document_type)
    .bind(&dto.document_number)
    .bind(&dto.phone)
    .bind(&dto.email)
    .bind(&dto.address)
    .bind(&dto.notes)
    .fetch_one(&mut *tx)
    .await?;

    let changes = changed_fields(&before, &after, &["id", "updated_at"]);
    if changes.is_empty() {
        return Ok(before);
    }

    record_event(
        &mut tx,
        actor,
        "update",
        after.id,
        format!("Cliente #{} actualizado", after.id),
        Some(JsonValue::Object(changes)),
        json!({ "name": after.name }),
    )
    .await?;

    tx.commit().await?;
    Ok(after)
}

pub async fn set_customer_active(
    pool: &PgPool,
    id: i32,
    dto: SetCustomerActiveDto,
) -> Result<Customer, CustomerError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;

    let before = lock_customer(&mut tx, id)
        .await?
        .ok_or(CustomerError::NotFound)?;
    if before.is_active == dto.is_active {
        return Ok(before);
    }

    let after = sqlx::query_as::<_, Customer>(&format!(
        "UPDATE customers SET is_active = $2 WHERE id = $1 RETURNING {COLS}"
    ))
    .bind(id)
    .bind(dto.is_active)
    .fetch_one(&mut *tx)
    .await?;

    let (action, label) = if dto.is_active {
        ("activate", "reactivado")
    } else {
        ("deactivate", "desactivado")
    };
    record_event(
        &mut tx,
        actor,
        action,
        after.id,
        format!("Cliente #{} {}", after.id, label),
        Some(json!({ "is_active": { "from": before.is_active, "to": after.is_active } })),
        json!({ "name": after.name }),
    )
    .await?;

    tx.commit().await?;
    Ok(after)
}

pub async fn get_overview(pool: &PgPool, id: i32) -> Result<Option<CustomerOverview>, sqlx::Error> {
    sqlx::query_as::<_, CustomerOverview>(&format!(
        "SELECT {COLS_C}, {OVERVIEW_STATS} {OVERVIEW_FROM} WHERE c.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

fn search_pattern(search: &Option<String>) -> Option<String> {
    let text = search.as_deref().map(str::trim).filter(|t| !t.is_empty())?;
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, filter: &CustomerFilterDto) {
    qb.push(" WHERE TRUE");
    if let Some(pattern) = search_pattern(&filter.search) {
        qb.push(" AND (c.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.document_number ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.phone ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.email ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(active) = filter.is_active {
        qb.push(" AND c.is_active = ").push_bind(active);
    }
}

pub async fn list_customers(
    pool: &PgPool,
    filter: &CustomerFilterDto,
) -> Result<(Vec<CustomerOverview>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(page_size);

    let mut rows_query =
        QueryBuilder::<Postgres>::new(format!("SELECT {COLS_C}, {OVERVIEW_STATS} {OVERVIEW_FROM}"));
    push_filters(&mut rows_query, filter);
    rows_query
        .push(" ORDER BY c.is_active DESC, lower(c.name) ASC, c.id ASC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows = rows_query
        .build_query_as::<CustomerOverview>()
        .fetch_all(pool)
        .await?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM customers c");
    push_filters(&mut count_query, filter);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    Ok((rows, total))
}

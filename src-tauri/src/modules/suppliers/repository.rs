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
use crate::modules::suppliers::{
    dto::{CreateSupplierDto, SetSupplierActiveDto, SupplierFilterDto, UpdateSupplierDto},
    models::{Supplier, SupplierOverview},
};

#[derive(Debug)]
pub enum SupplierError {
    Db(sqlx::Error),
    NotFound,
    DuplicateName,
    DuplicateDocument,
}

impl From<sqlx::Error> for SupplierError {
    fn from(error: sqlx::Error) -> Self {
        let constraint = error
            .as_database_error()
            .and_then(|db| db.constraint())
            .map(str::to_owned);
        match constraint.as_deref() {
            Some("uq_suppliers_name") => SupplierError::DuplicateName,
            Some("uq_suppliers_document") => SupplierError::DuplicateDocument,
            _ => SupplierError::Db(error),
        }
    }
}

const COLS: &str = "id, name, contact_name, document_type, document_number, phone, whatsapp, \
                    email, address, city, payment_terms_days, lead_time_days, visit_days, \
                    priority, notes, is_active, created_by, created_at, updated_at";
const OVERVIEW_COLS: &str = "s.id, s.name, s.contact_name, s.document_type, s.document_number,
    s.phone, s.whatsapp, s.email, s.address, s.city, s.payment_terms_days, s.lead_time_days,
    s.visit_days, s.priority, s.notes, s.is_active, s.created_by, s.created_at, s.updated_at,
    COALESCE(o.category_count, 0)::BIGINT AS category_count,
    COALESCE(o.variant_count, 0)::BIGINT AS variant_count,
    COALESCE(o.low_stock_count, 0)::BIGINT AS low_stock_count,
    COALESCE(o.out_of_stock_count, 0)::BIGINT AS out_of_stock_count";

fn overview_from() -> String {
    let stock_status = super::links::stock_status_case();
    format!(
        "FROM suppliers s
         LEFT JOIN LATERAL (
             SELECT
                 (SELECT COUNT(*) FROM supplier_categories sc
                  WHERE sc.supplier_id = s.id)::BIGINT AS category_count,
                 COUNT(sv.id)::BIGINT AS variant_count,
                 COUNT(sv.id) FILTER (WHERE {stock_status} = 'low')::BIGINT AS low_stock_count,
                 COUNT(sv.id) FILTER (WHERE {stock_status} = 'out_of_stock')::BIGINT
                     AS out_of_stock_count
             FROM supplier_variants sv
             JOIN product_variants v ON v.id = sv.variant_id
             JOIN products p ON p.id = v.product_id
             WHERE sv.supplier_id = s.id AND v.is_active AND p.is_active
         ) o ON TRUE"
    )
}

async fn record_event(
    conn: &mut sqlx::PgConnection,
    actor: Option<String>,
    action: &str,
    supplier_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> Result<(), sqlx::Error> {
    audit_repo::insert_event(
        conn,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Suppliers,
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("supplier".to_string()),
            entity_id: Some(supplier_id.to_string()),
            summary,
            changes,
            metadata: Some(metadata),
            error_message: None,
        },
    )
    .await?;
    Ok(())
}

async fn lock_supplier(
    conn: &mut sqlx::PgConnection,
    id: i32,
) -> Result<Option<Supplier>, sqlx::Error> {
    sqlx::query_as::<_, Supplier>(&format!(
        "SELECT {COLS} FROM suppliers WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn create_supplier(
    pool: &PgPool,
    dto: CreateSupplierDto,
) -> Result<Supplier, SupplierError> {
    let f = &dto.fields;
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let priority = f.priority.as_deref().unwrap_or("normal");
    let mut tx = pool.begin().await?;

    let supplier = sqlx::query_as::<_, Supplier>(&format!(
        "INSERT INTO suppliers
             (name, contact_name, document_type, document_number, phone, whatsapp, email,
              address, city, payment_terms_days, lead_time_days, visit_days, priority,
              notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         RETURNING {COLS}"
    ))
    .bind(&f.name)
    .bind(&f.contact_name)
    .bind(&f.document_type)
    .bind(&f.document_number)
    .bind(&f.phone)
    .bind(&f.whatsapp)
    .bind(&f.email)
    .bind(&f.address)
    .bind(&f.city)
    .bind(f.payment_terms_days)
    .bind(f.lead_time_days)
    .bind(&f.visit_days)
    .bind(priority)
    .bind(&f.notes)
    .bind(&created_by)
    .fetch_one(&mut *tx)
    .await?;

    record_event(
        &mut tx,
        actor,
        "create",
        supplier.id,
        format!("Proveedor #{} registrado: {}", supplier.id, supplier.name),
        None,
        json!({
            "name": supplier.name,
            "city": supplier.city,
            "priority": supplier.priority,
            "document_type": supplier.document_type,
            "document_number": supplier.document_number,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(supplier)
}

/// Si no cambia nada, no escribe ni audita.
pub async fn update_supplier(
    pool: &PgPool,
    id: i32,
    dto: UpdateSupplierDto,
) -> Result<Supplier, SupplierError> {
    let f = &dto.fields;
    let actor = declared_actor(&dto.updated_by);
    let priority = f.priority.as_deref().unwrap_or("normal");
    let mut tx = pool.begin().await?;

    let before = lock_supplier(&mut tx, id)
        .await?
        .ok_or(SupplierError::NotFound)?;

    let after = sqlx::query_as::<_, Supplier>(&format!(
        "UPDATE suppliers
         SET name = $2, contact_name = $3, document_type = $4, document_number = $5,
             phone = $6, whatsapp = $7, email = $8, address = $9, city = $10,
             payment_terms_days = $11, lead_time_days = $12, visit_days = $13,
             priority = $14, notes = $15
         WHERE id = $1
         RETURNING {COLS}"
    ))
    .bind(id)
    .bind(&f.name)
    .bind(&f.contact_name)
    .bind(&f.document_type)
    .bind(&f.document_number)
    .bind(&f.phone)
    .bind(&f.whatsapp)
    .bind(&f.email)
    .bind(&f.address)
    .bind(&f.city)
    .bind(f.payment_terms_days)
    .bind(f.lead_time_days)
    .bind(&f.visit_days)
    .bind(priority)
    .bind(&f.notes)
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
        format!("Proveedor #{} actualizado", after.id),
        Some(JsonValue::Object(changes)),
        json!({ "name": after.name }),
    )
    .await?;

    tx.commit().await?;
    Ok(after)
}

pub async fn set_supplier_active(
    pool: &PgPool,
    id: i32,
    dto: SetSupplierActiveDto,
) -> Result<Supplier, SupplierError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;

    let before = lock_supplier(&mut tx, id)
        .await?
        .ok_or(SupplierError::NotFound)?;
    if before.is_active == dto.is_active {
        return Ok(before);
    }

    let after = sqlx::query_as::<_, Supplier>(&format!(
        "UPDATE suppliers SET is_active = $2 WHERE id = $1 RETURNING {COLS}"
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
        format!("Proveedor #{} {}", after.id, label),
        Some(json!({ "is_active": { "from": before.is_active, "to": after.is_active } })),
        json!({ "name": after.name }),
    )
    .await?;

    tx.commit().await?;
    Ok(after)
}

pub async fn get_supplier(pool: &PgPool, id: i32) -> Result<Option<SupplierOverview>, sqlx::Error> {
    sqlx::query_as::<_, SupplierOverview>(&format!(
        "SELECT {OVERVIEW_COLS} {} WHERE s.id = $1",
        overview_from()
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

fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, filter: &SupplierFilterDto) {
    qb.push(" WHERE TRUE");
    if let Some(pattern) = search_pattern(&filter.search) {
        qb.push(" AND (s.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.contact_name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.document_number ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.phone ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.whatsapp ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.email ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.city ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(active) = filter.is_active {
        qb.push(" AND s.is_active = ").push_bind(active);
    }
    if let Some(priority) = filter.priority.as_deref().filter(|p| !p.is_empty()) {
        qb.push(" AND s.priority = ")
            .push_bind(priority.to_string());
    }
}

/// Activos primero; dentro de cada grupo: urgentes, preferidos y normales; luego por nombre.
pub async fn list_suppliers(
    pool: &PgPool,
    filter: &SupplierFilterDto,
) -> Result<(Vec<SupplierOverview>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(page_size);

    let mut rows_query =
        QueryBuilder::<Postgres>::new(format!("SELECT {OVERVIEW_COLS} {}", overview_from()));
    push_filters(&mut rows_query, filter);
    rows_query
        .push(
            " ORDER BY s.is_active DESC,
              CASE s.priority WHEN 'urgent' THEN 0 WHEN 'preferred' THEN 1 ELSE 2 END ASC,
              lower(s.name) ASC, s.id ASC LIMIT ",
        )
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows = rows_query
        .build_query_as::<SupplierOverview>()
        .fetch_all(pool)
        .await?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM suppliers s");
    push_filters(&mut count_query, filter);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    Ok((rows, total))
}

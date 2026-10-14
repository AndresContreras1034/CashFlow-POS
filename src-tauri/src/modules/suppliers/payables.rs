//! Deudas con un proveedor. SOLO LECTURA.
//!
//! El cruce con `debts` es por nombre del acreedor: la tabla de deudas no tiene
//! una FK hacia proveedores, por lo que se ignoran mayúsculas y espacios extremos.

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::debts::models::DebtOverview;
use crate::modules::suppliers::repository;

const ACTIVE: &str = "o.status IN ('pending', 'partially_paid', 'overdue')";
const DUE_SOON: &str = "o.urgency IN ('alert', 'due_today')";
const SAME_CREDITOR: &str = "lower(btrim(o.creditor_name)) = lower(btrim($1))";
const LIST_LIMIT: i64 = 20;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SupplierPayablesSummary {
    pub pending_total: i64,
    pub overdue_total: i64,
    pub due_soon_total: i64,
    pub active_count: i64,
    pub overdue_count: i64,
    pub next_due_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupplierPayables {
    pub matched_name: String,
    pub summary: SupplierPayablesSummary,
    pub debts: Vec<DebtOverview>,
    pub debts_truncated: bool,
}

pub async fn get_supplier_payables(
    pool: &PgPool,
    supplier_id: i32,
) -> Result<SupplierPayables, AppError> {
    let supplier = repository::get_supplier(pool, supplier_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Proveedor no encontrado"))?;

    let summary = sqlx::query_as::<_, SupplierPayablesSummary>(&format!(
        "SELECT
             COALESCE(SUM(o.balance) FILTER (WHERE {ACTIVE}), 0)::BIGINT AS pending_total,
             COALESCE(SUM(o.balance) FILTER (WHERE o.status = 'overdue'), 0)::BIGINT
                 AS overdue_total,
             COALESCE(SUM(o.balance) FILTER (WHERE {DUE_SOON}), 0)::BIGINT AS due_soon_total,
             (COUNT(*) FILTER (WHERE {ACTIVE}))::BIGINT AS active_count,
             (COUNT(*) FILTER (WHERE o.status = 'overdue'))::BIGINT AS overdue_count,
             MIN(o.due_on) FILTER (WHERE {ACTIVE} AND o.due_on >= o.today) AS next_due_on
         FROM debt_overview o
         WHERE {SAME_CREDITOR}"
    ))
    .bind(&supplier.name)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    let mut debts = sqlx::query_as::<_, DebtOverview>(&format!(
        "SELECT o.* FROM debt_overview o
         WHERE {SAME_CREDITOR} AND {ACTIVE}
         ORDER BY o.due_on ASC NULLS LAST, o.id DESC
         LIMIT $2"
    ))
    .bind(&supplier.name)
    .bind(LIST_LIMIT + 1)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    let debts_truncated = debts.len() as i64 > LIST_LIMIT;
    debts.truncate(LIST_LIMIT as usize);

    Ok(SupplierPayables {
        matched_name: supplier.name,
        summary,
        debts,
        debts_truncated,
    })
}

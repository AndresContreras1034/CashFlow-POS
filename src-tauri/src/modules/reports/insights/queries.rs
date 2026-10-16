use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::PgPool;

use super::super::repository::{ALLOCATED_ITEMS_CTES, COSTED_SALES_CTES};
use super::combos::Item;

async fn report_timezone(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT timezone FROM app_settings WHERE id = 1")
        .fetch_one(pool)
        .await
}

pub async fn fetch_star_variants(
    pool: &PgPool,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(i64, i64)>, sqlx::Error> {
    let timezone = report_timezone(pool).await?;
    let query = format!(
        r#"{COSTED_SALES_CTES}
           {ALLOCATED_ITEMS_CTES}
           SELECT
               a.variant_id::BIGINT AS variant_id,
               COALESCE(SUM(a.gross_alloc - a.tax_alloc), 0)::BIGINT AS revenue
           FROM allocated_items a
           GROUP BY a.variant_id
           ORDER BY revenue DESC, variant_id ASC
           LIMIT 10"#
    );

    sqlx::query_as::<_, (i64, i64)>(&query)
        .bind(from)
        .bind(to)
        .bind(timezone)
        .fetch_all(pool)
        .await
}

pub async fn fetch_units_28d(
    pool: &PgPool,
    from: NaiveDate,
    to: NaiveDate,
    variant_ids: &[i64],
) -> Result<HashMap<i64, i64>, sqlx::Error> {
    let timezone = report_timezone(pool).await?;
    let query = format!(
        r#"{COSTED_SALES_CTES}
           {ALLOCATED_ITEMS_CTES}
           SELECT
               a.variant_id::BIGINT AS variant_id,
               COALESCE(SUM(a.quantity), 0)::BIGINT AS units
           FROM allocated_items a
           WHERE a.variant_id::BIGINT = ANY($4)
           GROUP BY a.variant_id"#
    );
    let rows = sqlx::query_as::<_, (i64, i64)>(&query)
        .bind(from)
        .bind(to)
        .bind(timezone)
        .bind(variant_ids.to_vec())
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().collect())
}

pub async fn fetch_history_days(pool: &PgPool, today_local: NaiveDate) -> Result<i64, sqlx::Error> {
    let timezone = report_timezone(pool).await?;
    sqlx::query_scalar(
        r#"SELECT COALESCE(
               ($1::DATE - MIN((created_at AT TIME ZONE $2)::DATE))::BIGINT,
               0::BIGINT
           )
           FROM sales
           WHERE status = 'completed'"#,
    )
    .bind(today_local)
    .bind(timezone)
    .fetch_one(pool)
    .await
}

pub async fn fetch_uncosted_pct_bps(
    pool: &PgPool,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<i64, sqlx::Error> {
    let timezone = report_timezone(pool).await?;
    let query = format!(
        r#"{COSTED_SALES_CTES}
           SELECT COALESCE(
               COUNT(*) FILTER (WHERE courtesy_reason IS NULL AND NOT fully_costed) * 10_000
                   / NULLIF(COUNT(*) FILTER (WHERE courtesy_reason IS NULL), 0),
               0
           )::BIGINT
           FROM costed_sales"#
    );

    sqlx::query_scalar(&query)
        .bind(from)
        .bind(to)
        .bind(timezone)
        .fetch_one(pool)
        .await
}

pub async fn fetch_stock(
    pool: &PgPool,
    variant_ids: &[i64],
) -> Result<HashMap<i64, i64>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64, i64)>(
        r#"SELECT id::BIGINT, stock::BIGINT
           FROM product_variants
           WHERE id::BIGINT = ANY($1)"#,
    )
    .bind(variant_ids.to_vec())
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}

pub async fn fetch_combo_items(
    pool: &PgPool,
    variant_ids: &[i64],
) -> Result<Vec<Item>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"SELECT
               v.id::BIGINT,
               p.category_id::BIGINT,
               v.price::BIGINT,
               v.cost::BIGINT,
               (v.stock::NUMERIC * v.cost::NUMERIC)::BIGINT
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           WHERE v.id::BIGINT = ANY($1)"#,
    )
    .bind(variant_ids.to_vec())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(variant_id, category_id, price, cost, stock_value)| Item {
            variant_id,
            category_id,
            price,
            cost,
            stock_value,
        })
        .collect())
}

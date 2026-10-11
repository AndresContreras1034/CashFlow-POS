use sqlx::PgPool;

const COSTED_SALES_CTES: &str = r#"
    WITH filtered_sales AS (
        SELECT s.*
        FROM sales s
        WHERE s.status = 'completed'
          AND s.created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
          AND s.created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
    ),
    item_totals_by_sale AS (
        SELECT
            si.sale_id,
            BOOL_AND(si.unit_cost IS NOT NULL) AS fully_costed,
            COALESCE(
                SUM(si.unit_cost * si.quantity::BIGINT)
                    FILTER (WHERE si.unit_cost IS NOT NULL),
                0
            )::BIGINT AS known_cogs,
            COALESCE(SUM(si.quantity::BIGINT), 0)::BIGINT AS units_sold,
            COUNT(*) FILTER (WHERE si.unit_cost IS NULL)::BIGINT AS items_without_cost
        FROM sale_items si
        JOIN filtered_sales s ON s.id = si.sale_id
        GROUP BY si.sale_id
    ),
    costed_sales AS (
        SELECT
            s.id,
            s.created_at,
            s.total,
            s.tax,
            s.courtesy_reason,
            (items.sale_id IS NOT NULL AND items.fully_costed) AS fully_costed,
            COALESCE(items.known_cogs, 0)::BIGINT AS known_cogs,
            COALESCE(items.units_sold, 0)::BIGINT AS units_sold,
            COALESCE(items.items_without_cost, 0)::BIGINT AS items_without_cost
        FROM filtered_sales s
        LEFT JOIN item_totals_by_sale items ON items.sale_id = s.id
    )
"#;

use crate::{
    modules::{
        reports::dto::{DateRange, HeatmapCell, PaymentMethodStat, SalesKpis, TimeSeriesPoint},
        sales::models::PaymentMethod,
    },
    types::money::divide_to_currency,
};

#[derive(sqlx::FromRow)]
struct SalesTotals {
    tickets: i64,
    gross_sales: i64,
    discount_total: i64,
    tax_total: i64,
    cancelled_count: i64,
    refunded_count: i64,
    courtesy_count: i64,
    courtesy_value: i64,
}

#[derive(sqlx::FromRow)]
struct ItemTotals {
    cogs: i64,
    units_sold: i64,
    items_without_cost: i64,
    courtesy_cost: i64,
    cogs_costed_non_courtesy: i64,
    uncosted_tickets: i64,
    profit_base_net_sales: i64,
}

#[derive(sqlx::FromRow)]
struct SalesSeriesRow {
    bucket: chrono::NaiveDate,
    tickets: i64,
    gross_sales: i64,
    gross_profit: i64,
}

#[derive(sqlx::FromRow)]
struct PaymentMethodRow {
    method: PaymentMethod,
    tickets: i64,
    amount: i64,
    total_amount: i64,
}

#[derive(sqlx::FromRow)]
struct HeatmapRow {
    weekday: i16,
    hour: i16,
    tickets: i64,
    gross_sales: i64,
}

pub async fn fetch_sales_kpis(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
    currency_decimals: u8,
) -> Result<SalesKpis, sqlx::Error> {
    let sales = sqlx::query_as!(
        SalesTotals,
        r#"SELECT
               COUNT(*) FILTER (WHERE status = 'completed')::BIGINT AS "tickets!",
               COALESCE(
                   SUM(CASE WHEN courtesy_reason IS NULL THEN total ELSE 0 END)
                       FILTER (WHERE status = 'completed'),
                   0
               )::BIGINT AS "gross_sales!",
               COALESCE(
                   SUM(discount)
                       FILTER (WHERE status = 'completed' AND courtesy_reason IS NULL),
                   0
               )::BIGINT
                   AS "discount_total!",
               COALESCE(SUM(tax) FILTER (WHERE status = 'completed'), 0)::BIGINT AS "tax_total!",
               COUNT(*) FILTER (WHERE status = 'cancelled')::BIGINT AS "cancelled_count!",
               COUNT(*) FILTER (WHERE status = 'refunded')::BIGINT AS "refunded_count!",
               COUNT(*) FILTER (
                   WHERE status = 'completed' AND courtesy_reason IS NOT NULL
               )::BIGINT AS "courtesy_count!",
               COALESCE(
                   SUM(subtotal)
                       FILTER (WHERE status = 'completed' AND courtesy_reason IS NOT NULL),
                   0
               )::BIGINT AS "courtesy_value!"
           FROM sales
           WHERE created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
             AND created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)"#,
        range.from,
        range.to,
        timezone
    )
    .fetch_one(pool)
    .await?;

    let items_query = format!(
        r#"{COSTED_SALES_CTES}
           SELECT
               COALESCE(SUM(items.known_cogs), 0)::BIGINT AS "cogs",
               COALESCE(SUM(items.units_sold), 0)::BIGINT AS "units_sold",
               COALESCE(SUM(items.items_without_cost), 0)::BIGINT AS "items_without_cost",
               COALESCE(
                   SUM(items.known_cogs) FILTER (WHERE s.courtesy_reason IS NOT NULL),
                   0
               )::BIGINT AS "courtesy_cost",
               COALESCE(
                   SUM(s.known_cogs)
                       FILTER (
                           WHERE s.fully_costed AND s.courtesy_reason IS NULL
                       ),
                   0
               )::BIGINT AS "cogs_costed_non_courtesy",
               COUNT(*) FILTER (
                   WHERE items.sale_id IS NOT NULL
                     AND NOT s.fully_costed
                     AND s.courtesy_reason IS NULL
               )::BIGINT AS "uncosted_tickets",
               COALESCE(
                   SUM(s.total - s.tax)
                       FILTER (
                           WHERE s.fully_costed AND s.courtesy_reason IS NULL
                       ),
                   0
               )::BIGINT AS "profit_base_net_sales"
           FROM costed_sales s
           LEFT JOIN item_totals_by_sale items ON items.sale_id = s.id"#
    );
    let items = sqlx::query_as::<_, ItemTotals>(&items_query)
        .bind(range.from)
        .bind(range.to)
        .bind(timezone)
        .fetch_one(pool)
        .await?;

    let net_sales = sales.gross_sales - sales.tax_total;
    let gross_profit =
        items.profit_base_net_sales - items.cogs_costed_non_courtesy - items.courtesy_cost;
    let paid_tickets = sales.tickets - sales.courtesy_count;
    let avg_ticket = if paid_tickets == 0 {
        0
    } else {
        match divide_to_currency(sales.gross_sales, paid_tickets, currency_decimals) {
            Some(value) => value,
            None => 0,
        }
    };

    Ok(SalesKpis {
        tickets: sales.tickets,
        gross_sales: sales.gross_sales,
        discount_total: sales.discount_total,
        tax_total: sales.tax_total,
        net_sales,
        avg_ticket,
        cogs: items.cogs,
        gross_profit,
        margin_bps: super::range::margin_bps(gross_profit, items.profit_base_net_sales),
        units_sold: items.units_sold,
        items_without_cost: items.items_without_cost,
        cancelled_count: sales.cancelled_count,
        refunded_count: sales.refunded_count,
        courtesy_count: sales.courtesy_count,
        courtesy_cost: items.courtesy_cost,
        uncosted_tickets: items.uncosted_tickets,
        profit_base_net_sales: items.profit_base_net_sales,
        courtesy_value: sales.courtesy_value,
    })
}

pub async fn fetch_sales_series(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
    granularity: &str,
) -> Result<Vec<TimeSeriesPoint>, sqlx::Error> {
    let query = format!(
        r#"{COSTED_SALES_CTES}
           SELECT
               DATE_TRUNC($4::TEXT, s.created_at AT TIME ZONE $3)::DATE AS "bucket",
               COUNT(*)::BIGINT AS "tickets",
               COALESCE(SUM(s.total), 0)::BIGINT AS "gross_sales",
               COALESCE(
                   SUM(
                       CASE
                           WHEN s.fully_costed
                               THEN s.total - s.tax - s.known_cogs
                           ELSE 0
                       END
                   ),
                   0
               )::BIGINT AS "gross_profit"
           FROM costed_sales s
           WHERE s.courtesy_reason IS NULL
           GROUP BY 1
           ORDER BY 1 ASC"#
    );
    let rows = sqlx::query_as::<_, SalesSeriesRow>(&query)
        .bind(range.from)
        .bind(range.to)
        .bind(timezone)
        .bind(granularity)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| TimeSeriesPoint {
            bucket: row.bucket,
            tickets: row.tickets,
            gross_sales: row.gross_sales,
            gross_profit: row.gross_profit,
        })
        .collect())
}

pub async fn fetch_sales_by_payment_method(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
) -> Result<Vec<PaymentMethodStat>, sqlx::Error> {
    let rows = sqlx::query_as!(
        PaymentMethodRow,
        r#"SELECT
               sp.method AS "method: PaymentMethod",
               COUNT(DISTINCT sp.sale_id)::BIGINT AS "tickets!",
               COALESCE(SUM(sp.amount), 0)::BIGINT AS "amount!",
               SUM(SUM(sp.amount)) OVER ()::BIGINT AS "total_amount!"
           FROM sale_payments sp
           JOIN sales s ON s.id = sp.sale_id
           WHERE s.status = 'completed'
             AND s.courtesy_reason IS NULL
             AND s.created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
             AND s.created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
           GROUP BY sp.method"#,
        range.from,
        range.to,
        timezone
    )
    .fetch_all(pool)
    .await?;

    // Ticket counts are non-additive: a mixed-payment sale is counted for each method it uses.
    let total_amount = rows.first().map_or(0, |row| row.total_amount);
    Ok([
        PaymentMethod::Cash,
        PaymentMethod::Card,
        PaymentMethod::Transfer,
    ]
    .into_iter()
    .map(|method| {
        let row = rows.iter().find(|row| row.method == method);
        let amount = row.map_or(0, |row| row.amount);
        PaymentMethodStat {
            method,
            tickets: row.map_or(0, |row| row.tickets),
            amount,
            share_bps: super::range::share_bps(amount, total_amount),
        }
    })
    .collect())
}

pub async fn fetch_sales_heatmap(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
) -> Result<Vec<HeatmapCell>, sqlx::Error> {
    let rows = sqlx::query_as!(
        HeatmapRow,
        r#"SELECT
               (EXTRACT(ISODOW FROM s.created_at AT TIME ZONE $3) - 1)::SMALLINT AS "weekday!",
               EXTRACT(HOUR FROM s.created_at AT TIME ZONE $3)::SMALLINT AS "hour!",
               COUNT(*)::BIGINT AS "tickets!",
               COALESCE(SUM(s.total), 0)::BIGINT AS "gross_sales!"
           FROM sales s
           WHERE s.status = 'completed'
             AND s.courtesy_reason IS NULL
             AND s.created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
             AND s.created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
           GROUP BY 1, 2
           ORDER BY 1, 2"#,
        range.from,
        range.to,
        timezone
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| HeatmapCell {
            weekday: row.weekday as u8,
            hour: row.hour as u8,
            tickets: row.tickets,
            gross_sales: row.gross_sales,
        })
        .collect())
}

use sqlx::PgPool;

use crate::{
    modules::reports::dto::{DateRange, SalesKpis},
    types::money::divide_to_currency,
};

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

struct ItemTotals {
    cogs: i64,
    units_sold: i64,
    items_without_cost: i64,
    courtesy_cost: i64,
    cogs_costed_non_courtesy: i64,
    uncosted_tickets: i64,
    profit_base_net_sales: i64,
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

    let items = sqlx::query_as!(
        ItemTotals,
        r#"WITH item_totals_by_sale AS (
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
               JOIN sales s ON s.id = si.sale_id
               WHERE s.status = 'completed'
                 AND s.created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
                 AND s.created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
               GROUP BY si.sale_id
           )
           SELECT
               COALESCE(SUM(items.known_cogs), 0)::BIGINT AS "cogs!",
               COALESCE(SUM(items.units_sold), 0)::BIGINT AS "units_sold!",
               COALESCE(SUM(items.items_without_cost), 0)::BIGINT AS "items_without_cost!",
               COALESCE(
                   SUM(items.known_cogs) FILTER (WHERE s.courtesy_reason IS NOT NULL),
                   0
               )::BIGINT AS "courtesy_cost!",
               COALESCE(
                   SUM(items.known_cogs)
                       FILTER (
                           WHERE items.fully_costed AND s.courtesy_reason IS NULL
                       ),
                   0
               )::BIGINT AS "cogs_costed_non_courtesy!",
               COUNT(*) FILTER (
                   WHERE NOT items.fully_costed AND s.courtesy_reason IS NULL
               )::BIGINT AS "uncosted_tickets!",
               COALESCE(
                   SUM(s.total - s.tax)
                       FILTER (
                           WHERE items.fully_costed AND s.courtesy_reason IS NULL
                       ),
                   0
               )::BIGINT AS "profit_base_net_sales!"
           FROM item_totals_by_sale items
           JOIN sales s ON s.id = items.sale_id"#,
        range.from,
        range.to,
        timezone
    )
    .fetch_one(pool)
    .await?;

    let net_sales = sales.gross_sales - sales.tax_total;
    let gross_profit =
        items.profit_base_net_sales - items.cogs_costed_non_courtesy - items.courtesy_cost;
    let paid_tickets = sales.tickets - sales.courtesy_count;
    let avg_ticket = if paid_tickets == 0 {
        0
    } else {
        divide_to_currency(sales.gross_sales, paid_tickets, currency_decimals).unwrap_or(0)
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

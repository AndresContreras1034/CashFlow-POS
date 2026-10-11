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

const ALLOCATED_ITEMS_CTES: &str = r#",
    item_base AS (
        SELECT
            si.id AS item_id,
            si.sale_id,
            si.variant_id,
            si.quantity::BIGINT AS quantity,
            si.subtotal::NUMERIC AS item_subtotal,
            si.unit_cost,
            s.total::NUMERIC AS sale_total,
            s.tax::NUMERIC AS sale_tax,
            s.fully_costed,
            SUM(si.subtotal::NUMERIC) OVER (
                PARTITION BY si.sale_id ORDER BY si.id
                ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
            ) AS cumulative_subtotal,
            SUM(si.subtotal::NUMERIC) OVER (PARTITION BY si.sale_id) AS sale_subtotal
        FROM sale_items si
        JOIN costed_sales s ON s.id = si.sale_id
        WHERE s.courtesy_reason IS NULL
    ),
    gross_allocated_items AS (
        SELECT
            item_base.*,
            CASE
                WHEN sale_subtotal = 0 THEN 0::NUMERIC
                ELSE ROUND(cumulative_subtotal * sale_total / sale_subtotal)
                     - ROUND((cumulative_subtotal - item_subtotal) * sale_total / sale_subtotal)
            END AS gross_alloc
        FROM item_base
    ),
    allocated_items AS (
        SELECT
            gross_allocated_items.*,
            CASE
                WHEN sale_total = 0 THEN 0::NUMERIC
                ELSE ROUND(
                    SUM(gross_alloc) OVER (
                        PARTITION BY sale_id ORDER BY item_id
                        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                    ) * sale_tax / sale_total
                ) - ROUND(
                    (
                        SUM(gross_alloc) OVER (
                            PARTITION BY sale_id ORDER BY item_id
                            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                        ) - gross_alloc
                    ) * sale_tax / sale_total
                )
            END AS tax_alloc
        FROM gross_allocated_items
    )
"#;

use crate::{
    modules::{
        reports::dto::{
            CategorySales, DateRange, HeatmapCell, PaymentMethodStat, RankBy, SalesKpis,
            StaleProduct, TimeSeriesPoint, TopProduct,
        },
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

#[derive(sqlx::FromRow)]
struct ProductRankingRow {
    product_id: i32,
    variant_id: i32,
    product_name: String,
    attributes: serde_json::Value,
    units: i64,
    gross_revenue: i64,
    revenue: i64,
    profit: i64,
    costed_revenue: i64,
    uncosted_units: i64,
    units_total: i64,
    revenue_total: i64,
    profit_total: i64,
}

#[derive(sqlx::FromRow)]
struct CategoryRankingRow {
    category_id: i32,
    category_name: String,
    units: i64,
    gross_revenue: i64,
    revenue: i64,
    profit: i64,
    costed_revenue: i64,
    uncosted_units: i64,
    revenue_total: i64,
}

#[derive(sqlx::FromRow)]
struct StaleProductRow {
    variant_id: i32,
    product_name: String,
    attributes: serde_json::Value,
    stock: i32,
    stock_value_at_cost: i64,
    cost_missing: bool,
    days_since_last_sale: Option<i64>,
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

pub async fn fetch_top_products(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
    category_id: Option<i32>,
    limit: i64,
    rank_by: RankBy,
) -> Result<Vec<TopProduct>, sqlx::Error> {
    let order_by = match rank_by {
        RankBy::Units => "units DESC, variant_id ASC",
        RankBy::Revenue => "revenue DESC, variant_id ASC",
        RankBy::Profit => "profit DESC, variant_id ASC",
    };
    let query = format!(
        r#"{COSTED_SALES_CTES}
           {ALLOCATED_ITEMS_CTES},
           product_totals AS (
               SELECT
                   p.id AS product_id,
                   v.id AS variant_id,
                   p.name AS product_name,
                   v.attributes,
                   COALESCE(SUM(a.quantity), 0)::BIGINT AS units,
                   COALESCE(SUM(a.gross_alloc), 0)::BIGINT AS gross_revenue,
                   COALESCE(SUM(a.gross_alloc - a.tax_alloc), 0)::BIGINT AS revenue,
                   COALESCE(SUM(
                       CASE WHEN a.fully_costed
                           THEN a.gross_alloc - a.tax_alloc
                                - COALESCE(a.unit_cost::NUMERIC * a.quantity, 0)
                           ELSE 0
                       END
                   ), 0)::BIGINT AS profit,
                   COALESCE(SUM(a.gross_alloc - a.tax_alloc)
                       FILTER (WHERE a.fully_costed), 0)::BIGINT AS costed_revenue,
                   COALESCE(SUM(a.quantity)
                       FILTER (WHERE NOT a.fully_costed), 0)::BIGINT AS uncosted_units
               FROM allocated_items a
               JOIN product_variants v ON v.id = a.variant_id
               JOIN products p ON p.id = v.product_id
               WHERE ($4::INT IS NULL OR p.category_id = $4)
               GROUP BY p.id, v.id, p.name, v.attributes
           )
           -- The window totals are computed after category_id filtering, so shares are
           -- relative to the selected category whenever that filter is present.
           SELECT product_totals.*,
               SUM(units) OVER ()::BIGINT AS units_total,
               SUM(revenue) OVER ()::BIGINT AS revenue_total,
               SUM(profit) OVER ()::BIGINT AS profit_total
           FROM product_totals
           ORDER BY {order_by}
           LIMIT $5"#
    );
    let rows = sqlx::query_as::<_, ProductRankingRow>(&query)
        .bind(range.from)
        .bind(range.to)
        .bind(timezone)
        .bind(category_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let (share, total) = match rank_by {
                RankBy::Units => (row.units, row.units_total),
                RankBy::Revenue => (row.revenue, row.revenue_total),
                RankBy::Profit => (row.profit, row.profit_total),
            };
            TopProduct {
                product_id: row.product_id,
                variant_id: row.variant_id,
                product_name: row.product_name,
                attributes: row.attributes,
                units: row.units,
                gross_revenue: row.gross_revenue,
                revenue: row.revenue,
                profit: row.profit,
                margin_bps: (row.costed_revenue != 0)
                    .then(|| super::range::margin_bps(row.profit, row.costed_revenue)),
                uncosted_units: row.uncosted_units,
                share_bps: super::range::share_bps(share, total),
            }
        })
        .collect())
}

pub async fn fetch_sales_by_category(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
) -> Result<Vec<CategorySales>, sqlx::Error> {
    let query = format!(
        r#"{COSTED_SALES_CTES}
           {ALLOCATED_ITEMS_CTES},
           category_totals AS (
               SELECT
                   c.id AS category_id,
                   c.name AS category_name,
                   COALESCE(SUM(a.quantity), 0)::BIGINT AS units,
                   COALESCE(SUM(a.gross_alloc), 0)::BIGINT AS gross_revenue,
                   COALESCE(SUM(a.gross_alloc - a.tax_alloc), 0)::BIGINT AS revenue,
                   COALESCE(SUM(
                       CASE WHEN a.fully_costed
                           THEN a.gross_alloc - a.tax_alloc
                                - COALESCE(a.unit_cost::NUMERIC * a.quantity, 0)
                           ELSE 0
                       END
                   ), 0)::BIGINT AS profit,
                   COALESCE(SUM(a.gross_alloc - a.tax_alloc)
                       FILTER (WHERE a.fully_costed), 0)::BIGINT AS costed_revenue,
                   COALESCE(SUM(a.quantity)
                       FILTER (WHERE NOT a.fully_costed), 0)::BIGINT AS uncosted_units
               FROM allocated_items a
               JOIN product_variants v ON v.id = a.variant_id
               JOIN products p ON p.id = v.product_id
               JOIN categories c ON c.id = p.category_id
               GROUP BY c.id, c.name
           )
           SELECT category_totals.*,
               SUM(revenue) OVER ()::BIGINT AS revenue_total
           FROM category_totals
           ORDER BY revenue DESC, category_id ASC"#
    );
    let rows = sqlx::query_as::<_, CategoryRankingRow>(&query)
        .bind(range.from)
        .bind(range.to)
        .bind(timezone)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| CategorySales {
            category_id: row.category_id,
            category_name: row.category_name,
            units: row.units,
            gross_revenue: row.gross_revenue,
            revenue: row.revenue,
            profit: row.profit,
            margin_bps: (row.costed_revenue != 0)
                .then(|| super::range::margin_bps(row.profit, row.costed_revenue)),
            uncosted_units: row.uncosted_units,
            share_bps: super::range::share_bps(row.revenue, row.revenue_total),
        })
        .collect())
}

pub async fn fetch_stale_products(
    pool: &PgPool,
    range: &DateRange,
    timezone: &str,
    category_id: Option<i32>,
    limit: i64,
) -> Result<Vec<StaleProduct>, sqlx::Error> {
    let rows = sqlx::query_as!(
        StaleProductRow,
        r#"SELECT
               v.id AS "variant_id!",
               p.name AS "product_name!",
               v.attributes AS "attributes!",
               v.stock AS "stock!",
               (v.stock::NUMERIC * v.cost::NUMERIC)::BIGINT AS "stock_value_at_cost!",
               (v.cost = 0) AS "cost_missing!",
               (
                   $2::DATE - (
                       SELECT MAX((prior.created_at AT TIME ZONE $3)::DATE)
                       FROM sales prior
                       WHERE prior.status = 'completed'
                         AND prior.created_at < ($1::DATE::TIMESTAMP AT TIME ZONE $3)
                         AND EXISTS (
                             SELECT 1 FROM sale_items prior_item
                             WHERE prior_item.sale_id = prior.id
                               AND prior_item.variant_id = v.id
                         )
                   )
               )::BIGINT AS "days_since_last_sale?"
           FROM product_variants v
           JOIN products p ON p.id = v.product_id
           WHERE v.is_active
             AND p.is_active
             AND v.stock > 0
             AND ($4::INT IS NULL OR p.category_id = $4)
             AND v.created_at < ($1::DATE::TIMESTAMP AT TIME ZONE $3)
             AND NOT EXISTS (
                 SELECT 1
                 FROM sales in_range
                 JOIN sale_items range_item ON range_item.sale_id = in_range.id
                 WHERE in_range.status = 'completed'
                   AND range_item.variant_id = v.id
                   AND in_range.created_at >= ($1::DATE::TIMESTAMP AT TIME ZONE $3)
                   AND in_range.created_at < (($2::DATE + 1)::TIMESTAMP AT TIME ZONE $3)
             )
           ORDER BY 5 DESC, v.id ASC
           LIMIT $5"#,
        range.from,
        range.to,
        timezone,
        category_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| StaleProduct {
            variant_id: row.variant_id,
            product_name: row.product_name,
            attributes: row.attributes,
            stock: row.stock,
            stock_value_at_cost: row.stock_value_at_cost,
            cost_missing: row.cost_missing,
            days_since_last_sale: row.days_since_last_sale,
        })
        .collect())
}

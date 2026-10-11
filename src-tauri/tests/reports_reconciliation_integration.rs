use chrono::{DateTime, NaiveDate, Utc};
use pos_lib::modules::{
    debts::{dto::DebtFilterDto, service as debt_service},
    inventory::service as inventory_service,
    reports::{
        dto::{Granularity, RankBy, ReportFilter},
        service as report_service,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn filter(from: &str, to: &str) -> ReportFilter {
    ReportFilter {
        date_from: date(from),
        date_to: date(to),
        granularity: Some(Granularity::Day),
        compare_previous: false,
        category_id: None,
        limit: Some(100),
        rank_by: Some(RankBy::Revenue),
    }
}

async fn seed_variant(pool: &PgPool) -> i32 {
    let suffix = Uuid::new_v4().simple().to_string();
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(format!("reconciliation-category-{suffix}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 =
        sqlx::query_scalar("INSERT INTO products (category_id, name) VALUES ($1, $2) RETURNING id")
            .bind(category_id)
            .bind(format!("reconciliation-product-{suffix}"))
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, stock_min)
           VALUES ($1, '{"size":"test"}', 10000, 3000, 5, 1)
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn insert_completed_sale(pool: &PgPool, variant_id: i32, courtesy: bool) {
    let (subtotal, discount, total, created_at, courtesy_reason) = if courtesy {
        (
            5_000_i64,
            5_000_i64,
            0_i64,
            "2026-10-09T17:00:00Z",
            Some("Courtesy test"),
        )
    } else {
        (10_000_i64, 0_i64, 10_000_i64, "2026-10-09T16:00:00Z", None)
    };
    let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(created_at)
        .unwrap()
        .with_timezone(&Utc);
    let sale_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO sales
               (subtotal, tax, discount, total, status, created_by, created_at,
                courtesy_reason, gross_semantics)
           VALUES ($1, 0, $2, $3, 'completed', 'reports-reconciliation', $4, $5, TRUE)
           RETURNING id"#,
    )
    .bind(subtotal)
    .bind(discount)
    .bind(total)
    .bind(created_at)
    .bind(courtesy_reason)
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"INSERT INTO sale_items
               (sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal)
           VALUES ($1, $2, 1, $3, 3000, 0, 0, $3)"#,
    )
    .bind(sale_id)
    .bind(variant_id)
    .bind(subtotal)
    .execute(pool)
    .await
    .unwrap();

    if !courtesy {
        sqlx::query("INSERT INTO sale_payments (sale_id, method, amount) VALUES ($1, 'cash', $2)")
            .bind(sale_id)
            .bind(total)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn seed_debt(pool: &PgPool) {
    sqlx::query(
        r#"INSERT INTO debts
               (creditor_name, category, nature, concept, original_amount, issued_on,
                due_on, idempotency_key)
           VALUES ('Reconciliation creditor', 'maintenance', 'operating',
                   'Reconciliation debt', 25000, '2026-10-09', '2026-11-01', $1)"#,
    )
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .unwrap();
}

fn empty_filter() -> ReportFilter {
    filter("2026-10-09", "2026-10-09")
}

#[sqlx::test(migrations = "./migrations")]
async fn report_totals_reconcile_with_inventory_and_payables(pool: PgPool) {
    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();
    let variant_id = seed_variant(&pool).await;
    insert_completed_sale(&pool, variant_id, false).await;
    insert_completed_sale(&pool, variant_id, true).await;
    seed_debt(&pool).await;

    let report_filter = filter("2026-10-09", "2026-10-09");
    let dashboard = report_service::get_dashboard_kpis(&pool, report_filter.clone())
        .await
        .unwrap();
    let top_products = report_service::get_top_products(&pool, report_filter.clone())
        .await
        .unwrap();
    let payments = report_service::get_sales_by_payment_method(&pool, report_filter.clone())
        .await
        .unwrap();
    let courtesy_cost: i64 = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(si.unit_cost * si.quantity::BIGINT), 0)::BIGINT
           FROM sale_items si
           JOIN sales s ON s.id = si.sale_id
           WHERE s.status = 'completed'
             AND s.courtesy_reason IS NOT NULL"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        dashboard.current.gross_sales,
        top_products
            .iter()
            .map(|row| row.gross_revenue)
            .sum::<i64>()
    );
    assert_eq!(
        dashboard.current.gross_sales,
        payments.iter().map(|row| row.amount).sum::<i64>()
    );
    assert_eq!(
        dashboard.current.gross_profit,
        top_products.iter().map(|row| row.profit).sum::<i64>() - courtesy_cost
    );

    let valuation = report_service::get_inventory_valuation(&pool, report_filter.clone())
        .await
        .unwrap();
    let inventory_value = inventory_service::get_inventory_value(&pool).await.unwrap();
    assert_eq!(valuation.value_at_cost, inventory_value.value_at_cost);

    let payables = report_service::get_payables_summary(&pool, report_filter)
        .await
        .unwrap();
    let debt_summary = debt_service::get_summary(&pool, DebtFilterDto::default())
        .await
        .unwrap();
    assert_eq!(payables.pending_total, debt_summary.pending_total);
}

#[sqlx::test(migrations = "./migrations")]
async fn empty_database_returns_zero_metrics_and_empty_collections(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'America/Bogota' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let report_filter = empty_filter();

    let dashboard = report_service::get_dashboard_kpis(&pool, report_filter.clone())
        .await
        .unwrap();
    assert_eq!(dashboard.current.tickets, 0);
    assert_eq!(dashboard.current.gross_sales, 0);
    assert_eq!(dashboard.current.gross_profit, 0);
    let series = report_service::get_sales_series(&pool, report_filter.clone())
        .await
        .unwrap();
    assert!(series
        .iter()
        .all(|bucket| bucket.tickets == 0 && bucket.gross_sales == 0));
    let payment_methods = report_service::get_sales_by_payment_method(&pool, report_filter.clone())
        .await
        .unwrap();
    assert!(payment_methods
        .iter()
        .all(|stat| stat.amount == 0 && stat.tickets == 0));
    assert!(
        report_service::get_sales_heatmap(&pool, report_filter.clone())
            .await
            .unwrap()
            .iter()
            .all(|cell| cell.tickets == 0 && cell.gross_sales == 0)
    );
    assert!(
        report_service::get_top_products(&pool, report_filter.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        report_service::get_sales_by_category(&pool, report_filter.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        report_service::get_stale_products(&pool, report_filter.clone())
            .await
            .unwrap()
            .is_empty()
    );

    let valuation = report_service::get_inventory_valuation(&pool, report_filter.clone())
        .await
        .unwrap();
    assert_eq!(valuation.value_at_cost, 0);
    assert_eq!(valuation.potential_sale_value, 0);
    assert_eq!(valuation.units_in_stock, 0);
    assert!(valuation.by_category.is_empty());
    let turnover = report_service::get_inventory_turnover(&pool, report_filter.clone())
        .await
        .unwrap();
    assert_eq!(turnover.cogs, 0);
    assert_eq!(turnover.inventory_value_at_cost, 0);
    assert_eq!(turnover.turnover_bps, None);
    assert_eq!(turnover.days_of_inventory, None);
    assert!(
        report_service::get_stock_alerts(&pool, report_filter.clone())
            .await
            .unwrap()
            .is_empty()
    );

    let payables = report_service::get_payables_summary(&pool, report_filter.clone())
        .await
        .unwrap();
    assert_eq!(payables.pending_total, 0);
    assert_eq!(payables.active_count, 0);
    assert_eq!(payables.paid_in_range, 0);
    let aging = report_service::get_payables_aging(&pool, report_filter.clone())
        .await
        .unwrap();
    assert_eq!(aging.len(), 6);
    assert!(aging
        .iter()
        .all(|bucket| bucket.count == 0 && bucket.balance_total == 0));
    assert!(
        report_service::get_top_creditors(&pool, report_filter.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(report_service::get_upcoming_dues(&pool, report_filter)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn report_filter_accepts_one_and_366_days_and_rejects_367(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'UTC' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let one_day = report_service::get_dashboard_kpis(&pool, filter("2026-10-09", "2026-10-09"))
        .await
        .unwrap();
    assert_eq!(one_day.current.tickets, 0);

    let max_range = report_service::get_dashboard_kpis(&pool, filter("2026-01-01", "2027-01-01"))
        .await
        .unwrap();
    assert_eq!(max_range.current.tickets, 0);

    let error = report_service::get_dashboard_kpis(&pool, filter("2026-01-01", "2027-01-02"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("366 días"));
}

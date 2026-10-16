use chrono::{DateTime, NaiveDate, Utc};
use pos_lib::modules::reports::{
    dto::{DateRange, ReportFilter},
    service,
};
use pos_lib::modules::sales::models::PaymentMethod;
use sqlx::PgPool;

async fn seed_variant(pool: &PgPool) -> i32 {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(format!("reports-category-{suffix}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 =
        sqlx::query_scalar("INSERT INTO products (category_id, name) VALUES ($1, $2) RETURNING id")
            .bind(category_id)
            .bind(format!("reports-product-{suffix}"))
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price) \
         VALUES ($1, '{}'::jsonb, 1) RETURNING id",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_inventory_variant(
    pool: &PgPool,
    category_name: &str,
    product_name: &str,
    stock: i32,
    stock_min: i32,
    cost: i64,
    price: i64,
    variant_active: bool,
    product_active: bool,
) -> (i32, i32, i32) {
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(category_name)
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name, is_active) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(category_id)
    .bind(product_name)
    .bind(product_active)
    .fetch_one(pool)
    .await
    .unwrap();
    let variant_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, stock_min, is_active)
           VALUES ($1, '{"variant":"test"}', $2, $3, $4, $5, $6)
           RETURNING id"#,
    )
    .bind(product_id)
    .bind(price)
    .bind(cost)
    .bind(stock)
    .bind(stock_min)
    .bind(variant_active)
    .fetch_one(pool)
    .await
    .unwrap();
    (category_id, product_id, variant_id)
}

async fn insert_sale(
    pool: &PgPool,
    variant_id: i32,
    status: &str,
    created_at: &str,
    subtotal: i64,
    tax: i64,
    discount: i64,
    total: i64,
    courtesy_reason: Option<&str>,
    item_quantity: i32,
    item_unit_price: i64,
    item_discount: i64,
    item_tax: i64,
    item_subtotal: i64,
    unit_cost: Option<i64>,
) -> i32 {
    let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(created_at)
        .unwrap()
        .with_timezone(&Utc);
    let is_voided = status != "completed";
    let voided_at = is_voided.then_some(created_at);

    let sale_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO sales
               (subtotal, tax, discount, total, status, created_by, created_at,
                courtesy_reason, gross_semantics, voided_at, void_reason, voided_by)
           VALUES ($1, $2, $3, $4, $5::sale_status, 'reports-test', $6,
                   $7, TRUE, $8, $9, $10)
           RETURNING id"#,
    )
    .bind(subtotal)
    .bind(tax)
    .bind(discount)
    .bind(total)
    .bind(status)
    .bind(created_at)
    .bind(courtesy_reason)
    .bind(voided_at)
    .bind(is_voided.then_some("test void"))
    .bind(is_voided.then_some("reports-test"))
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"INSERT INTO sale_items
               (sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(sale_id)
    .bind(variant_id)
    .bind(item_quantity)
    .bind(item_unit_price)
    .bind(unit_cost)
    .bind(item_discount)
    .bind(item_tax)
    .bind(item_subtotal)
    .execute(pool)
    .await
    .unwrap();

    sale_id
}

async fn insert_payments(pool: &PgPool, sale_id: i32, payments: &[(&str, i64)]) {
    for (method, amount) in payments {
        sqlx::query(
            "INSERT INTO sale_payments (sale_id, method, amount) \
             VALUES ($1, $2::payment_method, $3)",
        )
        .bind(sale_id)
        .bind(method)
        .bind(amount)
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn insert_mixed_cost_sale(pool: &PgPool, variant_id: i32) {
    let sale_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO sales
               (subtotal, tax, discount, total, status, created_by, created_at, gross_semantics)
           VALUES (20000, 0, 0, 20000, 'completed', 'reports-test', '2026-10-09T17:00:00Z', TRUE)
           RETURNING id"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"INSERT INTO sale_items
               (sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal)
           VALUES ($1, $2, 1, 10000, 2000, 0, 0, 10000),
                  ($1, $2, 1, 10000, NULL, 0, 0, 10000)"#,
    )
    .bind(sale_id)
    .bind(variant_id)
    .execute(pool)
    .await
    .unwrap();
}

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_kpis_include_costs_void_counts_and_local_date_comparison(pool: PgPool) {
    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let variant_id = seed_variant(&pool).await;

    // 2026-10-09 local, normal completed sale.
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T15:00:00Z",
        14_000,
        1_900,
        2_100,
        11_900,
        None,
        2,
        7_000,
        500,
        1_900,
        13_500,
        Some(4_000),
    )
    .await;

    // Courtesy sale contributes cost and units but no sales or tax.
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T16:00:00Z",
        10_000,
        0,
        10_000,
        0,
        Some("Apoyo"),
        1,
        10_000,
        0,
        0,
        10_000,
        Some(3_000),
    )
    .await;

    // 02:30 UTC on October 10 is 21:30 in Colombia on October 9.
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-10T02:30:00Z",
        11_900,
        1_900,
        0,
        11_900,
        None,
        1,
        11_900,
        0,
        1_900,
        11_900,
        None,
    )
    .await;

    insert_sale(
        &pool,
        variant_id,
        "cancelled",
        "2026-10-09T18:00:00Z",
        5_000,
        0,
        0,
        5_000,
        None,
        1,
        5_000,
        0,
        0,
        5_000,
        Some(1_000),
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "refunded",
        "2026-10-09T19:00:00Z",
        6_000,
        0,
        0,
        6_000,
        None,
        1,
        6_000,
        0,
        0,
        6_000,
        Some(2_000),
    )
    .await;

    // Previous local day, with a lower completed sale.
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-08T15:00:00Z",
        10_000,
        1_597,
        0,
        10_000,
        None,
        1,
        10_000,
        0,
        0,
        10_000,
        Some(2_000),
    )
    .await;

    let report = service::get_dashboard_kpis(
        &pool,
        ReportFilter {
            date_from: date("2026-10-09"),
            date_to: date("2026-10-09"),
            granularity: None,
            compare_previous: true,
            compare_year_over_year: false,
            category_id: None,
            limit: None,
            rank_by: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        report.range,
        DateRange {
            from: date("2026-10-09"),
            to: date("2026-10-09"),
        }
    );
    assert_eq!(
        report.previous_range,
        Some(DateRange {
            from: date("2026-10-08"),
            to: date("2026-10-08"),
        })
    );
    assert_eq!(
        report.current,
        pos_lib::modules::reports::dto::SalesKpis {
            tickets: 2,
            gross_sales: 23_800,
            discount_total: 2_100,
            tax_total: 3_800,
            net_sales: 20_000,
            avg_ticket: 11_900,
            cogs: 8_000,
            gross_profit: -1_000,
            margin_bps: -1_000,
            units_sold: 4,
            items_without_cost: 1,
            cancelled_count: 1,
            refunded_count: 1,
            courtesy_count: 1,
            courtesy_cost: 3_000,
            uncosted_tickets: 1,
            profit_base_net_sales: 10_000,
            courtesy_value: 10_000,
        }
    );
    assert_eq!(
        report.previous,
        Some(pos_lib::modules::reports::dto::SalesKpis {
            tickets: 1,
            gross_sales: 10_000,
            discount_total: 0,
            tax_total: 1_597,
            net_sales: 8_403,
            avg_ticket: 10_000,
            cogs: 2_000,
            gross_profit: 6_403,
            margin_bps: 7_620,
            units_sold: 1,
            items_without_cost: 0,
            cancelled_count: 0,
            refunded_count: 0,
            courtesy_count: 0,
            courtesy_cost: 0,
            uncosted_tickets: 0,
            profit_base_net_sales: 8_403,
            courtesy_value: 0,
        })
    );
    assert_eq!(
        report.gross_sales_delta,
        pos_lib::modules::reports::dto::Delta {
            current: 23_800,
            previous: Some(10_000),
            change_bps: Some(13_800),
        }
    );
    assert_eq!(
        report.gross_profit_delta,
        pos_lib::modules::reports::dto::Delta {
            current: -1_000,
            previous: Some(6_403),
            change_bps: Some(-11_562),
        }
    );
    assert_eq!(
        report.tickets_delta,
        pos_lib::modules::reports::dto::Delta {
            current: 2,
            previous: Some(1),
            change_bps: Some(10_000),
        }
    );
    assert_eq!(
        report.avg_ticket_delta,
        pos_lib::modules::reports::dto::Delta {
            current: 11_900,
            previous: Some(10_000),
            change_bps: Some(1_900),
        }
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_kpis_are_zero_for_empty_range_and_comparison_is_optional(pool: PgPool) {
    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let base_filter = ReportFilter {
        date_from: date("2026-10-09"),
        date_to: date("2026-10-09"),
        granularity: None,
        compare_previous: true,
        compare_year_over_year: false,
        category_id: None,
        limit: None,
        rank_by: None,
    };
    let compared = service::get_dashboard_kpis(&pool, base_filter.clone())
        .await
        .unwrap();

    assert_eq!(compared.current, zero_kpis());
    assert_eq!(compared.previous, Some(zero_kpis()));
    assert_eq!(compared.gross_sales_delta.change_bps, None);
    assert_eq!(compared.gross_profit_delta.change_bps, None);
    assert_eq!(compared.tickets_delta.change_bps, None);
    assert_eq!(compared.avg_ticket_delta.change_bps, None);

    let mut un_compared_filter = base_filter;
    un_compared_filter.compare_previous = false;
    let without_comparison = service::get_dashboard_kpis(&pool, un_compared_filter)
        .await
        .unwrap();

    assert_eq!(without_comparison.current, zero_kpis());
    assert_eq!(without_comparison.previous, None);
    assert_eq!(without_comparison.previous_range, None);
    assert_eq!(without_comparison.gross_sales_delta.previous, None);
    assert_eq!(without_comparison.gross_sales_delta.change_bps, None);
    assert_eq!(without_comparison.gross_profit_delta.previous, None);
    assert_eq!(without_comparison.gross_profit_delta.change_bps, None);
    assert_eq!(without_comparison.tickets_delta.previous, None);
    assert_eq!(without_comparison.tickets_delta.change_bps, None);
    assert_eq!(without_comparison.avg_ticket_delta.previous, None);
    assert_eq!(without_comparison.avg_ticket_delta.change_bps, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn customer_share_includes_walk_in_sales_and_average_excludes_courtesies(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'UTC' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let customer_id: i32 =
        sqlx::query_scalar("INSERT INTO customers (name) VALUES ('Reports customer') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (_, _, variant_id) = seed_inventory_variant(
        &pool,
        "customer-share",
        "Customer share product",
        1,
        0,
        100,
        300,
        true,
        true,
    )
    .await;

    let paid_sale = insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T12:00:00Z",
        30_000,
        0,
        0,
        30_000,
        None,
        1,
        30_000,
        0,
        0,
        30_000,
        Some(10_000),
    )
    .await;
    let courtesy_sale = insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T13:00:00Z",
        30_000,
        0,
        30_000,
        0,
        Some("Apoyo"),
        1,
        30_000,
        0,
        0,
        30_000,
        Some(10_000),
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T14:00:00Z",
        70_000,
        0,
        0,
        70_000,
        None,
        1,
        70_000,
        0,
        0,
        70_000,
        Some(10_000),
    )
    .await;
    sqlx::query("UPDATE sales SET customer_id = $1 WHERE id IN ($2, $3)")
        .bind(customer_id)
        .bind(paid_sale)
        .bind(courtesy_sale)
        .execute(&pool)
        .await
        .unwrap();

    let filter = ReportFilter {
        date_from: date("2026-10-09"),
        date_to: date("2026-10-09"),
        granularity: None,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: None,
        limit: Some(10),
        rank_by: None,
    };
    let top = service::get_top_customers(&pool, filter.clone())
        .await
        .unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].sales_count, 1);
    assert_eq!(top[0].avg_ticket, 30_000);
    assert_eq!(top[0].share_bps, 3_000);

    let split = service::get_customer_sales_split(&pool, filter)
        .await
        .unwrap();
    assert_eq!(split.len(), 1);
    assert_eq!(split[0].sales_count, 1);
    assert_eq!(split[0].share_bps, 3_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_kpis_reject_invalid_timezone(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'Invalid/Timezone' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let error = service::get_dashboard_kpis(
        &pool,
        ReportFilter {
            date_from: date("2026-10-09"),
            date_to: date("2026-10-09"),
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: None,
            limit: None,
            rank_by: None,
        },
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("zona horaria configurada no es válida"));
}

#[sqlx::test(migrations = "./migrations")]
async fn mixed_cost_ticket_is_excluded_from_profit_base(pool: PgPool) {
    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let variant_id = seed_variant(&pool).await;
    insert_mixed_cost_sale(&pool, variant_id).await;

    let report = service::get_dashboard_kpis(
        &pool,
        ReportFilter {
            date_from: date("2026-10-09"),
            date_to: date("2026-10-09"),
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: None,
            limit: None,
            rank_by: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(report.current.tickets, 1);
    assert_eq!(report.current.gross_sales, 20_000);
    assert_eq!(report.current.cogs, 2_000);
    assert_eq!(report.current.items_without_cost, 1);
    assert_eq!(report.current.uncosted_tickets, 1);
    assert_eq!(report.current.profit_base_net_sales, 0);
    assert_eq!(report.current.gross_profit, 0);
    assert_eq!(report.current.margin_bps, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn sales_breakdowns_use_local_buckets_and_exclude_non_sales(pool: PgPool) {
    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let variant_id = seed_variant(&pool).await;
    let oct_seventh_sale = insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-07T15:00:00Z",
        5_000,
        0,
        0,
        5_000,
        None,
        1,
        5_000,
        0,
        0,
        5_000,
        Some(1_000),
    )
    .await;
    insert_payments(&pool, oct_seventh_sale, &[("transfer", 5_000)]).await;

    let friday_sale = insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-10T02:30:00Z",
        10_000,
        0,
        0,
        10_000,
        None,
        1,
        10_000,
        0,
        0,
        10_000,
        Some(4_000),
    )
    .await;
    insert_payments(&pool, friday_sale, &[("cash", 6_000), ("card", 4_000)]).await;

    let cancelled = insert_sale(
        &pool,
        variant_id,
        "cancelled",
        "2026-10-09T18:00:00Z",
        4_000,
        0,
        0,
        4_000,
        None,
        1,
        4_000,
        0,
        0,
        4_000,
        Some(1_000),
    )
    .await;
    insert_payments(&pool, cancelled, &[("cash", 4_000)]).await;

    let refunded = insert_sale(
        &pool,
        variant_id,
        "refunded",
        "2026-10-09T19:00:00Z",
        3_000,
        0,
        0,
        3_000,
        None,
        1,
        3_000,
        0,
        0,
        3_000,
        Some(1_000),
    )
    .await;
    insert_payments(&pool, refunded, &[("card", 3_000)]).await;

    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T20:00:00Z",
        8_000,
        0,
        8_000,
        0,
        Some("Regalo"),
        1,
        8_000,
        0,
        0,
        8_000,
        Some(2_000),
    )
    .await;

    let filter = |granularity| ReportFilter {
        date_from: date("2026-10-07"),
        date_to: date("2026-10-10"),
        granularity,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: None,
        limit: None,
        rank_by: None,
    };

    let daily = service::get_sales_series(
        &pool,
        filter(Some(pos_lib::modules::reports::dto::Granularity::Day)),
    )
    .await
    .unwrap();
    assert_eq!(daily.len(), 4);
    assert_eq!(daily[0].bucket, date("2026-10-07"));
    assert_eq!(daily[0].tickets, 1);
    assert_eq!(daily[0].gross_sales, 5_000);
    assert_eq!(daily[0].gross_profit, 4_000);
    assert_eq!(daily[1].bucket, date("2026-10-08"));
    assert_eq!(daily[1].tickets, 0);
    assert_eq!(daily[2].bucket, date("2026-10-09"));
    assert_eq!(daily[2].tickets, 1);
    assert_eq!(daily[2].gross_sales, 10_000);
    assert_eq!(daily[2].gross_profit, 6_000);
    assert_eq!(daily[3].bucket, date("2026-10-10"));
    assert_eq!(daily[3].tickets, 0);

    let payments = service::get_sales_by_payment_method(&pool, filter(None))
        .await
        .unwrap();
    assert_eq!(
        payments.iter().map(|stat| stat.method).collect::<Vec<_>>(),
        vec![
            PaymentMethod::Cash,
            PaymentMethod::Card,
            PaymentMethod::Transfer
        ]
    );
    assert_eq!(payments[0].tickets, 1);
    assert_eq!(payments[0].amount, 6_000);
    assert_eq!(payments[0].share_bps, 4_000);
    assert_eq!(payments[1].tickets, 1);
    assert_eq!(payments[1].amount, 4_000);
    assert_eq!(payments[1].share_bps, 2_667);
    assert_eq!(payments[2].tickets, 1);
    assert_eq!(payments[2].amount, 5_000);
    assert_eq!(payments[2].share_bps, 3_333);
    assert_eq!(
        payments.iter().map(|stat| stat.share_bps).sum::<i32>(),
        10_000
    );

    let heatmap = service::get_sales_heatmap(&pool, filter(None))
        .await
        .unwrap();
    assert_eq!(heatmap.len(), 2);
    assert!(heatmap.iter().any(|cell| {
        cell.weekday == 4 && cell.hour == 21 && cell.tickets == 1 && cell.gross_sales == 10_000
    }));
    assert!(heatmap.iter().any(|cell| {
        cell.weekday == 2 && cell.hour == 10 && cell.tickets == 1 && cell.gross_sales == 5_000
    }));
}

#[sqlx::test(migrations = "./migrations")]
async fn long_sales_ranges_use_monthly_buckets_and_same_period_last_year(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'UTC' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let variant_id = seed_variant(&pool).await;
    for (created_at, amount) in [
        ("2024-11-20T12:00:00Z", 5_000),
        ("2025-02-03T12:00:00Z", 10_000),
        ("2025-11-20T12:00:00Z", 10_000),
        ("2026-02-03T12:00:00Z", 20_000),
    ] {
        insert_sale(
            &pool,
            variant_id,
            "completed",
            created_at,
            amount,
            0,
            0,
            amount,
            None,
            1,
            amount,
            0,
            0,
            amount,
            Some(amount / 2),
        )
        .await;
    }
    let filter = ReportFilter {
        date_from: date("2025-10-20"),
        date_to: date("2026-02-03"),
        granularity: Some(pos_lib::modules::reports::dto::Granularity::Day),
        compare_previous: true,
        compare_year_over_year: true,
        category_id: None,
        limit: None,
        rank_by: None,
    };

    let series = service::get_sales_series(&pool, filter.clone())
        .await
        .unwrap();
    assert_eq!(
        series.iter().map(|point| point.bucket).collect::<Vec<_>>(),
        vec![
            date("2025-10-01"),
            date("2025-11-01"),
            date("2025-12-01"),
            date("2026-01-01"),
            date("2026-02-01"),
        ]
    );
    assert_eq!(
        series.iter().map(|point| point.partial).collect::<Vec<_>>(),
        vec![true, false, false, false, true]
    );
    assert_eq!(series[1].gross_sales, 10_000);
    assert_eq!(series[4].gross_sales, 20_000);

    let dashboard = service::get_dashboard_kpis(&pool, filter).await.unwrap();
    assert_eq!(
        dashboard.previous_range,
        Some(DateRange {
            from: date("2024-10-20"),
            to: date("2025-02-03"),
        })
    );
    assert_eq!(dashboard.current.gross_sales, 30_000);
    assert_eq!(dashboard.previous.unwrap().gross_sales, 15_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn product_rankings_reconcile_allocations_costing_and_stale_stock(pool: PgPool) {
    use pos_lib::modules::reports::dto::RankBy;

    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let first_variant = seed_variant(&pool).await;
    let product_id: i32 =
        sqlx::query_scalar("SELECT product_id FROM product_variants WHERE id = $1")
            .bind(first_variant)
            .fetch_one(&pool)
            .await
            .unwrap();
    let second_variant: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, created_at)
           VALUES ($1, '{"size":"large"}', 2000, 500, 8, '2026-01-01T00:00:00Z')
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let stale_cancelled_variant: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, created_at)
           VALUES ($1, '{"color":"blue"}', 1000, 0, 5, '2026-01-01T00:00:00Z')
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let stale_sold_variant: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, created_at)
           VALUES ($1, '{"color":"red"}', 1000, 700, 2, '2026-01-01T00:00:00Z')
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let courtesy_only_variant: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, created_at)
           VALUES ($1, '{"color":"green"}', 2000, 1000, 3, '2026-01-01T00:00:00Z')
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let high_units_variant: i32 = sqlx::query_scalar(
        r#"INSERT INTO product_variants
               (product_id, attributes, price, cost, stock, created_at)
           VALUES ($1, '{"color":"yellow"}', 2500, 5000, 4, '2026-01-01T00:00:00Z')
           RETURNING id"#,
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query(
        "UPDATE product_variants SET stock = 10, created_at = '2026-01-01T00:00:00Z' \
         WHERE id = $1",
    )
    .bind(first_variant)
    .execute(&pool)
    .await
    .unwrap();

    let current_sale = insert_sale(
        &pool,
        first_variant,
        "completed",
        "2026-10-10T02:30:00Z",
        11_001,
        1_001,
        1_000,
        10_001,
        None,
        1,
        6_000,
        0,
        0,
        6_000,
        Some(1_000),
    )
    .await;
    sqlx::query(
        r#"INSERT INTO sale_items
               (sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal)
           VALUES ($1, $2, 2, 2000, 500, 0, 0, 4000)"#,
    )
    .bind(current_sale)
    .bind(second_variant)
    .execute(&pool)
    .await
    .unwrap();

    let historical_sale = insert_sale(
        &pool,
        second_variant,
        "completed",
        "2026-10-09T15:00:00Z",
        5_000,
        500,
        500,
        4_500,
        None,
        1,
        5_000,
        0,
        0,
        5_000,
        Some(1_000),
    )
    .await;
    sqlx::query("UPDATE sales SET gross_semantics = FALSE WHERE id = $1")
        .bind(historical_sale)
        .execute(&pool)
        .await
        .unwrap();

    insert_mixed_cost_sale(&pool, first_variant).await;

    insert_sale(
        &pool,
        high_units_variant,
        "completed",
        "2026-10-09T16:00:00Z",
        10_000,
        0,
        0,
        10_000,
        None,
        4,
        2_500,
        0,
        0,
        10_000,
        Some(5_000),
    )
    .await;

    insert_sale(
        &pool,
        stale_cancelled_variant,
        "cancelled",
        "2026-10-09T18:00:00Z",
        1_000,
        0,
        0,
        1_000,
        None,
        1,
        1_000,
        0,
        0,
        1_000,
        Some(100),
    )
    .await;
    insert_sale(
        &pool,
        courtesy_only_variant,
        "completed",
        "2026-10-09T19:00:00Z",
        2_000,
        0,
        2_000,
        0,
        Some("Cortesía"),
        1,
        2_000,
        0,
        0,
        2_000,
        Some(2_000),
    )
    .await;
    insert_sale(
        &pool,
        stale_sold_variant,
        "completed",
        "2026-09-20T12:00:00Z",
        1_000,
        0,
        0,
        1_000,
        None,
        1,
        1_000,
        0,
        0,
        1_000,
        Some(700),
    )
    .await;

    let filter = ReportFilter {
        date_from: date("2026-10-09"),
        date_to: date("2026-10-09"),
        granularity: None,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: None,
        limit: Some(100),
        rank_by: Some(RankBy::Revenue),
    };
    let dashboard = service::get_dashboard_kpis(&pool, filter.clone())
        .await
        .unwrap();
    let products = service::get_top_products(&pool, filter.clone())
        .await
        .unwrap();
    let categories = service::get_sales_by_category(&pool, filter.clone())
        .await
        .unwrap();

    assert_eq!(dashboard.current.gross_sales, 44_501);
    assert_eq!(
        products.iter().map(|p| p.gross_revenue).sum::<i64>(),
        44_501
    );
    assert_eq!(
        products.iter().map(|p| p.revenue).sum::<i64>(),
        dashboard.current.gross_sales - dashboard.current.tax_total
    );
    assert_eq!(
        products.iter().map(|p| p.profit).sum::<i64>() - dashboard.current.courtesy_cost,
        dashboard.current.gross_profit
    );
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].gross_revenue, dashboard.current.gross_sales);
    assert_eq!(
        categories[0].profit - dashboard.current.courtesy_cost,
        dashboard.current.gross_profit
    );
    assert_eq!(
        products
            .iter()
            .find(|p| p.variant_id == first_variant)
            .unwrap()
            .gross_revenue,
        26_001
    );
    assert_eq!(
        products
            .iter()
            .find(|p| p.variant_id == first_variant)
            .unwrap()
            .revenue,
        25_400
    );
    assert_eq!(
        products
            .iter()
            .find(|p| p.variant_id == second_variant)
            .unwrap()
            .gross_revenue,
        8_500
    );
    assert_eq!(
        products
            .iter()
            .find(|p| p.variant_id == second_variant)
            .unwrap()
            .revenue,
        7_600
    );
    assert_eq!(
        products
            .iter()
            .find(|p| p.variant_id == first_variant)
            .unwrap()
            .uncosted_units,
        2
    );
    assert!(!products
        .iter()
        .any(|product| product.variant_id == stale_cancelled_variant));
    assert_eq!(
        products
            .iter()
            .map(|product| product.share_bps)
            .sum::<i32>(),
        10_000
    );

    let mut units_filter = filter.clone();
    units_filter.rank_by = Some(RankBy::Units);
    let units_ranked = service::get_top_products(&pool, units_filter)
        .await
        .unwrap();
    assert_eq!(units_ranked[0].variant_id, high_units_variant);

    let mut profit_filter = filter.clone();
    profit_filter.rank_by = Some(RankBy::Profit);
    let profit_ranked = service::get_top_products(&pool, profit_filter)
        .await
        .unwrap();
    assert_eq!(profit_ranked[0].variant_id, second_variant);
    assert_eq!(products[0].variant_id, first_variant);

    let mut limited_filter = filter.clone();
    limited_filter.limit = Some(1);
    let limited = service::get_top_products(&pool, limited_filter)
        .await
        .unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].variant_id, first_variant);
    assert_eq!(limited[0].share_bps, products[0].share_bps);

    let mut category_filter = filter.clone();
    category_filter.category_id = Some(-1);
    assert_eq!(
        service::get_sales_by_category(&pool, category_filter)
            .await
            .unwrap()
            .len(),
        1
    );

    let stale = service::get_stale_products(&pool, filter).await.unwrap();
    assert_eq!(stale.len(), 2);
    assert!(stale.iter().any(|product| {
        product.variant_id == stale_cancelled_variant
            && product.cost_missing
            && product.days_since_last_sale.is_none()
    }));
    assert!(stale.iter().any(|product| {
        product.variant_id == stale_sold_variant
            && !product.cost_missing
            && product.days_since_last_sale == Some(19)
    }));
    assert!(!stale
        .iter()
        .any(|product| product.variant_id == courtesy_only_variant));
}

#[sqlx::test(migrations = "./migrations")]
async fn top_products_category_filter_keeps_share_based_on_all_period_sales(pool: PgPool) {
    use pos_lib::modules::reports::dto::RankBy;

    sqlx::query(
        "UPDATE app_settings SET timezone = 'America/Bogota', currency_decimals = 2 WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let first_variant = seed_variant(&pool).await;
    let second_variant = seed_variant(&pool).await;
    let first_category: i32 =
        sqlx::query_scalar("SELECT p.category_id FROM product_variants v JOIN products p ON p.id = v.product_id WHERE v.id = $1")
            .bind(first_variant)
            .fetch_one(&pool)
            .await
            .unwrap();
    let second_category: i32 =
        sqlx::query_scalar("SELECT p.category_id FROM product_variants v JOIN products p ON p.id = v.product_id WHERE v.id = $1")
            .bind(second_variant)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(first_category, second_category);

    for (variant_id, total) in [(first_variant, 8_000), (second_variant, 12_000)] {
        insert_sale(
            &pool,
            variant_id,
            "completed",
            "2026-10-09T15:00:00Z",
            total,
            0,
            0,
            total,
            None,
            1,
            total,
            0,
            0,
            total,
            Some(total / 2),
        )
        .await;
    }

    let filtered = service::get_top_products(
        &pool,
        ReportFilter {
            date_from: date("2026-10-09"),
            date_to: date("2026-10-09"),
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: Some(first_category),
            limit: Some(100),
            rank_by: Some(RankBy::Revenue),
        },
    )
    .await
    .unwrap();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].variant_id, first_variant);
    assert_eq!(filtered[0].share_bps, 4_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn stale_product_totals_include_all_rows_without_being_limited_to_the_display_page(
    pool: PgPool,
) {
    let date = Utc::now().date_naive() + chrono::Duration::days(1);
    let mut expected_value = 0_i64;
    let mut expected_without_cost = 0_i64;

    for index in 0..15 {
        let cost = if index < 8 {
            i64::from(index + 1) * 100
        } else {
            expected_without_cost += 1;
            0
        };
        expected_value += cost;
        seed_inventory_variant(
            &pool,
            &format!("stale-category-{index}"),
            &format!("stale-product-{index}"),
            1,
            0,
            cost,
            1_000,
            true,
            true,
        )
        .await;
    }

    let stale = service::get_stale_products(
        &pool,
        ReportFilter {
            date_from: date,
            date_to: date,
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: None,
            limit: Some(10),
            rank_by: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(stale.len(), 10);
    assert_eq!(stale[0].total_rows, 15);
    assert_eq!(stale[0].total_value_at_cost, expected_value);
    assert_eq!(stale[0].total_without_cost, expected_without_cost);
}

#[sqlx::test(migrations = "./migrations")]
async fn inventory_valuation_is_a_current_active_stock_snapshot(pool: PgPool) {
    let (category_a, _, _) = seed_inventory_variant(
        &pool,
        "valuation-a",
        "Valuation A",
        10,
        2,
        200,
        500,
        true,
        true,
    )
    .await;
    let (category_no_cost, _, _) = seed_inventory_variant(
        &pool,
        "valuation-a-2",
        "Valuation no cost",
        3,
        1,
        0,
        300,
        true,
        true,
    )
    .await;
    let (category_b, _, _) = seed_inventory_variant(
        &pool,
        "valuation-b",
        "Valuation B",
        4,
        1,
        100,
        250,
        true,
        true,
    )
    .await;
    let (_, _, negative_variant) = seed_inventory_variant(
        &pool,
        "valuation-negative",
        "Negative stock",
        0,
        0,
        300,
        500,
        true,
        true,
    )
    .await;
    sqlx::query("UPDATE product_variants SET stock = -2, allow_negative = TRUE WHERE id = $1")
        .bind(negative_variant)
        .execute(&pool)
        .await
        .unwrap();
    seed_inventory_variant(
        &pool,
        "valuation-inactive",
        "Inactive variant",
        20,
        0,
        100,
        200,
        false,
        true,
    )
    .await;
    seed_inventory_variant(
        &pool,
        "valuation-product-inactive",
        "Inactive product",
        30,
        0,
        100,
        200,
        true,
        false,
    )
    .await;

    let filter = ReportFilter {
        date_from: date("2026-01-01"),
        date_to: date("2026-01-01"),
        granularity: None,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: None,
        limit: Some(1),
        rank_by: None,
    };
    let valuation = service::get_inventory_valuation(&pool, filter.clone())
        .await
        .unwrap();
    let legacy = pos_lib::modules::inventory::service::get_inventory_value(&pool)
        .await
        .unwrap();

    assert_eq!(valuation.value_at_cost, 2_400);
    assert_eq!(valuation.potential_sale_value, 6_000);
    assert_eq!(valuation.potential_margin_bps, Some(6_000));
    assert_eq!(valuation.variants_valued, 3);
    assert_eq!(valuation.variants_without_cost, 1);
    assert_eq!(valuation.negative_stock_variants, 1);
    assert_eq!(valuation.units_in_stock, 17);
    assert_eq!(legacy.value_at_cost, valuation.value_at_cost);
    assert_eq!(legacy.potential_sale_value, valuation.potential_sale_value);
    assert_eq!(
        valuation
            .by_category
            .iter()
            .map(|category| category.value_at_cost)
            .sum::<i64>(),
        valuation.value_at_cost
    );
    assert_eq!(
        valuation
            .by_category
            .iter()
            .map(|category| category.category_id)
            .collect::<Vec<_>>(),
        vec![category_a, category_b, category_no_cost]
    );
    let no_cost_category = valuation
        .by_category
        .iter()
        .find(|category| category.category_id == category_no_cost)
        .unwrap();
    assert_eq!(no_cost_category.units, 3);
    assert_eq!(no_cost_category.value_at_cost, 0);
    assert_eq!(no_cost_category.variants_without_cost, 1);
    assert!(valuation.by_category.iter().any(|category| {
        category.category_id == category_a
            && category.units == 10
            && category.value_at_cost == 2_000
    }));

    let filtered = service::get_inventory_valuation(
        &pool,
        ReportFilter {
            category_id: Some(category_a),
            ..filter
        },
    )
    .await
    .unwrap();
    assert_eq!(filtered.value_at_cost, 2_000);
    assert_eq!(filtered.units_in_stock, 10);
    assert_eq!(filtered.by_category.len(), 1);
    assert_eq!(filtered.by_category[0].category_id, category_a);
}

#[sqlx::test(migrations = "./migrations")]
async fn inventory_turnover_uses_local_dates_and_excludes_uncosted_and_courtesy(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'America/Bogota' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let (category_id, _, variant_id) = seed_inventory_variant(
        &pool,
        "turnover",
        "Turnover product",
        10,
        2,
        200,
        500,
        true,
        true,
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-10T02:30:00Z",
        1_500,
        0,
        0,
        1_500,
        None,
        3,
        500,
        0,
        0,
        1_500,
        Some(200),
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T17:00:00Z",
        500,
        0,
        0,
        500,
        None,
        1,
        500,
        0,
        0,
        500,
        None,
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T18:00:00Z",
        1_000,
        0,
        1_000,
        0,
        Some("Cortesía"),
        2,
        500,
        0,
        0,
        1_000,
        Some(200),
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-01T15:00:00Z",
        1_000,
        0,
        0,
        1_000,
        None,
        2,
        500,
        0,
        0,
        1_000,
        Some(200),
    )
    .await;

    let make_filter = |from: &str, to: &str| ReportFilter {
        date_from: date(from),
        date_to: date(to),
        granularity: None,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: Some(category_id),
        limit: None,
        rank_by: None,
    };
    let one_day = service::get_inventory_turnover(&pool, make_filter("2026-10-09", "2026-10-09"))
        .await
        .unwrap();
    assert_eq!(one_day.cogs, 600);
    assert_eq!(one_day.inventory_value_at_cost, 2_000);
    assert_eq!(one_day.turnover_bps, Some(3_000));
    assert_eq!(one_day.days_of_inventory, None);
    assert_eq!(
        one_day.days_of_inventory_reason,
        Some(pos_lib::modules::reports::inventory_days::InsufficientReason::RangeTooShort),
    );
    assert_eq!(one_day.range_days, 1);
    assert_eq!(one_day.uncosted_items, 1);

    let thirty_days =
        service::get_inventory_turnover(&pool, make_filter("2026-10-01", "2026-10-30"))
            .await
            .unwrap();
    assert_eq!(thirty_days.cogs, 1_000);
    assert_eq!(thirty_days.days_of_inventory, Some(60));
    assert_eq!(thirty_days.range_days, 30);

    let no_cogs = service::get_inventory_turnover(&pool, make_filter("2026-09-01", "2026-09-28"))
        .await
        .unwrap();
    assert_eq!(no_cogs.cogs, 0);
    assert_eq!(no_cogs.turnover_bps, Some(0));
    assert_eq!(no_cogs.days_of_inventory, None);
    assert_eq!(
        no_cogs.days_of_inventory_reason,
        Some(pos_lib::modules::reports::inventory_days::InsufficientReason::NoCogs),
    );

    sqlx::query("UPDATE product_variants SET stock = 0 WHERE id = $1")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();
    let no_inventory =
        service::get_inventory_turnover(&pool, make_filter("2026-10-09", "2026-10-09"))
            .await
            .unwrap();
    assert_eq!(no_inventory.inventory_value_at_cost, 0);
    assert_eq!(no_inventory.turnover_bps, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn stock_alerts_order_status_cover_and_reorder_by_selected_range(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'America/Bogota' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let (category_id, _, out_sold_variant) =
        seed_inventory_variant(&pool, "alert-a", "Out sold", 0, 0, 100, 300, true, true).await;
    let (_, _, out_more_sold_variant) = seed_inventory_variant(
        &pool,
        "alert-more-sold",
        "Out more sold",
        0,
        0,
        100,
        300,
        true,
        true,
    )
    .await;
    let (_, _, out_never_variant) = seed_inventory_variant(
        &pool,
        "alert-never",
        "Out never",
        0,
        0,
        100,
        300,
        true,
        true,
    )
    .await;
    let (_, _, low_variant) =
        seed_inventory_variant(&pool, "alert-low", "Low stock", 2, 5, 100, 300, true, true).await;
    let (_, _, normal_variant) = seed_inventory_variant(
        &pool,
        "alert-normal",
        "Normal stock",
        6,
        5,
        100,
        300,
        true,
        true,
    )
    .await;
    let (_, _, inactive_variant) = seed_inventory_variant(
        &pool,
        "alert-inactive",
        "Inactive alert",
        0,
        0,
        100,
        300,
        false,
        true,
    )
    .await;

    insert_sale(
        &pool,
        out_sold_variant,
        "completed",
        "2026-10-10T02:30:00Z",
        300,
        0,
        0,
        300,
        None,
        1,
        300,
        0,
        0,
        300,
        Some(100),
    )
    .await;
    insert_sale(
        &pool,
        out_more_sold_variant,
        "completed",
        "2026-10-10T02:30:00Z",
        12_000,
        0,
        0,
        12_000,
        None,
        40,
        300,
        0,
        0,
        12_000,
        Some(100),
    )
    .await;
    insert_sale(
        &pool,
        low_variant,
        "completed",
        "2026-10-09T17:00:00Z",
        1_200,
        0,
        0,
        1_200,
        None,
        4,
        300,
        0,
        0,
        1_200,
        Some(100),
    )
    .await;
    insert_sale(
        &pool,
        low_variant,
        "completed",
        "2026-10-01T17:00:00Z",
        600,
        0,
        0,
        600,
        None,
        2,
        300,
        0,
        0,
        600,
        Some(100),
    )
    .await;

    let filter = ReportFilter {
        date_from: date("2026-10-09"),
        date_to: date("2026-10-09"),
        granularity: None,
        compare_previous: false,
        compare_year_over_year: false,
        category_id: None,
        limit: Some(25),
        rank_by: None,
    };
    let alerts = service::get_stock_alerts(&pool, filter.clone())
        .await
        .unwrap();
    assert_eq!(
        alerts
            .iter()
            .map(|alert| alert.variant_id)
            .collect::<Vec<_>>(),
        vec![
            out_more_sold_variant,
            out_sold_variant,
            out_never_variant,
            low_variant,
        ]
    );
    assert!(alerts
        .iter()
        .all(|alert| alert.variant_id != normal_variant));
    assert!(alerts
        .iter()
        .all(|alert| alert.variant_id != inactive_variant));
    assert_eq!(
        alerts[0].status,
        pos_lib::modules::inventory::models::StockStatus::OutOfStock
    );
    assert_eq!(alerts[0].units_sold, 40);
    assert_eq!(alerts[0].days_of_cover, Some(0));
    assert_eq!(alerts[0].suggested_reorder, 40);
    assert_eq!(alerts[1].units_sold, 1);
    assert_eq!(alerts[1].days_of_cover, Some(0));
    assert_eq!(alerts[1].suggested_reorder, 1);
    assert_eq!(alerts[2].days_of_cover, None);
    assert_eq!(alerts[2].suggested_reorder, 0);
    assert_eq!(
        alerts[3].status,
        pos_lib::modules::inventory::models::StockStatus::Low
    );
    assert_eq!(alerts[3].days_of_cover, Some(1));
    assert_eq!(alerts[3].suggested_reorder, 2);

    let mut month_filter = filter.clone();
    month_filter.date_from = date("2026-10-01");
    month_filter.date_to = date("2026-10-30");
    month_filter.limit = Some(100);
    let month_alerts = service::get_stock_alerts(&pool, month_filter)
        .await
        .unwrap();
    let monthly_low = month_alerts
        .iter()
        .find(|alert| alert.variant_id == low_variant)
        .unwrap();
    assert_eq!(monthly_low.units_sold, 6);
    assert_eq!(monthly_low.days_of_cover, Some(10));
    assert_eq!(monthly_low.suggested_reorder, 4);

    let mut limited_filter = filter.clone();
    limited_filter.limit = Some(1);
    assert_eq!(
        service::get_stock_alerts(&pool, limited_filter)
            .await
            .unwrap()
            .len(),
        1
    );
    let mut category_filter = filter;
    category_filter.category_id = Some(category_id);
    let category_alerts = service::get_stock_alerts(&pool, category_filter)
        .await
        .unwrap();
    assert_eq!(category_alerts.len(), 1);
    assert_eq!(category_alerts[0].variant_id, out_sold_variant);
}

#[sqlx::test(migrations = "./migrations")]
async fn inventory_turnover_counts_historical_cogs_after_product_deactivation(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'America/Bogota' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let (category_id, product_id, variant_id) = seed_inventory_variant(
        &pool,
        "turnover-inactive",
        "Deactivated after sale",
        8,
        1,
        250,
        500,
        true,
        true,
    )
    .await;
    insert_sale(
        &pool,
        variant_id,
        "completed",
        "2026-10-09T17:00:00Z",
        1_500,
        0,
        0,
        1_500,
        None,
        3,
        500,
        0,
        0,
        1_500,
        Some(250),
    )
    .await;
    sqlx::query("UPDATE products SET is_active = FALSE WHERE id = $1")
        .bind(product_id)
        .execute(&pool)
        .await
        .unwrap();

    let result = service::get_inventory_turnover(
        &pool,
        ReportFilter {
            date_from: date("2026-10-09"),
            date_to: date("2026-10-09"),
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: Some(category_id),
            limit: None,
            rank_by: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(result.cogs, 750);
    assert_eq!(result.inventory_value_at_cost, 0);
}

fn zero_kpis() -> pos_lib::modules::reports::dto::SalesKpis {
    pos_lib::modules::reports::dto::SalesKpis {
        tickets: 0,
        gross_sales: 0,
        discount_total: 0,
        tax_total: 0,
        net_sales: 0,
        avg_ticket: 0,
        cogs: 0,
        gross_profit: 0,
        margin_bps: 0,
        units_sold: 0,
        items_without_cost: 0,
        cancelled_count: 0,
        refunded_count: 0,
        courtesy_count: 0,
        courtesy_cost: 0,
        uncosted_tickets: 0,
        profit_base_net_sales: 0,
        courtesy_value: 0,
    }
}

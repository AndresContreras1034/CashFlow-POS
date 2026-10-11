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
            category_id: None,
            limit: None,
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
            tickets: 3,
            gross_sales: 23_800,
            discount_total: 2_100,
            tax_total: 3_800,
            net_sales: 20_000,
            avg_ticket: 11_900,
            cogs: 11_000,
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
            current: 3,
            previous: Some(1),
            change_bps: Some(20_000),
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
        category_id: None,
        limit: None,
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
            category_id: None,
            limit: None,
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
            category_id: None,
            limit: None,
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
        category_id: None,
        limit: None,
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

    let weekly = service::get_sales_series(
        &pool,
        filter(Some(pos_lib::modules::reports::dto::Granularity::Week)),
    )
    .await
    .unwrap();
    assert_eq!(weekly.len(), 1);
    assert_eq!(weekly[0].bucket, date("2026-10-05"));
    assert_eq!(weekly[0].tickets, 2);
    assert_eq!(weekly[0].gross_sales, 15_000);
    assert_eq!(weekly[0].gross_profit, 10_000);

    let monthly = service::get_sales_series(
        &pool,
        filter(Some(pos_lib::modules::reports::dto::Granularity::Month)),
    )
    .await
    .unwrap();
    assert_eq!(monthly.len(), 1);
    assert_eq!(monthly[0].bucket, date("2026-10-01"));
    assert_eq!(monthly[0].tickets, 2);
    assert_eq!(monthly[0].gross_sales, 15_000);

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

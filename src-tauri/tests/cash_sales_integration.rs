use pos_lib::modules::{
    cash::{dto::CloseSessionDto, service as cash_service},
    sales::{
        dto::{CreateSaleDto, CreateSaleItemDto, CreateSalePaymentDto},
        models::PaymentMethod,
        service as sales_service,
    },
};
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(1);

async fn seed(pool: &PgPool, price: i64, stock: i32) -> i32 {
    let suffix = SEQ.fetch_add(1, Ordering::Relaxed);
    let category: i32 = sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
        .bind(format!("cash-test-{suffix}")).fetch_one(pool).await.unwrap();
    let product: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name, is_active) VALUES ($1, 'cash-test', TRUE) RETURNING id",
    ).bind(category).fetch_one(pool).await.unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price, stock, is_active) \
         VALUES ($1, '{}'::jsonb, $2, $3, TRUE) RETURNING id",
    ).bind(product).bind(price).bind(stock).fetch_one(pool).await.unwrap()
}

async fn open(pool: &PgPool, amount: i64) -> i32 {
    sqlx::query_scalar("INSERT INTO cash_sessions (opening_amount) VALUES ($1) RETURNING id")
        .bind(amount).fetch_one(pool).await.unwrap()
}

fn sale(variant: i32, payments: Vec<CreateSalePaymentDto>) -> CreateSaleDto {
    CreateSaleDto {
        customer_id: None,
        items: vec![CreateSaleItemDto { variant_id: variant, quantity: 1, discount: None }],
        payments,
        discount: None,
        cash_received: None,
        notes: None,
        courtesy_reason: None,
        created_by: Some("integration-test".to_string()),
        idempotency_key: None,
    }
}

fn payment(method: PaymentMethod, amount: i64) -> CreateSalePaymentDto {
    CreateSalePaymentDto { method, amount }
}

#[sqlx::test(migrations = "./migrations")]
async fn cierre_incluye_venta_en_efectivo_por_neto_con_vuelto(pool: PgPool) {
    let variant = seed(&pool, 10_000, 5).await;
    let session = open(&pool, 5_000).await;
    let mut dto = sale(variant, vec![payment(PaymentMethod::Cash, 10_000)]);
    dto.cash_received = Some(20_000);
    sales_service::create_sale(&pool, dto).await.unwrap();
    let closed = cash_service::close_session(&pool, session, CloseSessionDto {
        counted_amount: 15_000, closing_notes: None, closed_by: Some("test".to_string()),
    }).await.unwrap();
    assert_eq!(closed.expected_amount, Some(15_000));
}

#[sqlx::test(migrations = "./migrations")]
async fn pago_mixto_solo_su_parte_efectivo_entra_a_caja(pool: PgPool) {
    let variant = seed(&pool, 10_000, 5).await;
    let session = open(&pool, 0).await;
    let sale_detail = sale(variant, vec![
        payment(PaymentMethod::Card, 4_000), payment(PaymentMethod::Cash, 6_000),
    ]);
    let created = sales_service::create_sale(&pool, sale_detail).await.unwrap();
    let movement: i64 = sqlx::query_scalar(
        "SELECT amount FROM cash_movements WHERE session_id = $1 AND movement_type = 'sale_in'",
    ).bind(session).fetch_one(&pool).await.unwrap();
    assert_eq!(movement, 6_000);
    assert_eq!(created.sale.total, 10_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn cortesia_y_tarjeta_no_crean_movimiento_de_caja(pool: PgPool) {
    let v1 = seed(&pool, 10_000, 5).await;
    let v2 = seed(&pool, 10_000, 5).await;
    let session = open(&pool, 0).await;
    let mut courtesy = sale(v1, vec![]);
    courtesy.discount = Some(10_000);
    courtesy.courtesy_reason = Some("Apoyo".to_string());
    sales_service::create_sale(&pool, courtesy).await.unwrap();
    sales_service::create_sale(&pool, sale(v2, vec![payment(PaymentMethod::Card, 10_000)]))
        .await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cash_movements WHERE session_id = $1 AND movement_type IN ('sale_in', 'sale_out')",
    ).bind(session).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn conciliacion_efectivo_de_pagos_igual_a_sale_in(pool: PgPool) {
    let v1 = seed(&pool, 10_000, 5).await;
    let v2 = seed(&pool, 10_000, 5).await;
    let session = open(&pool, 0).await;
    sales_service::create_sale(&pool, sale(v1, vec![payment(PaymentMethod::Cash, 10_000)]))
        .await.unwrap();
    sales_service::create_sale(&pool, sale(v2, vec![
        payment(PaymentMethod::Card, 4_000), payment(PaymentMethod::Cash, 6_000),
    ])).await.unwrap();
    let totals: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COALESCE(SUM(sp.amount), 0)::BIGINT FROM sale_payments sp \
             JOIN sales s ON s.id = sp.sale_id WHERE s.cash_session_id = $1 \
             AND s.status = 'completed' AND sp.method = 'cash'), \
         (SELECT COALESCE(SUM(amount), 0)::BIGINT FROM cash_movements \
             WHERE session_id = $1 AND movement_type = 'sale_in')",
    ).bind(session).fetch_one(&pool).await.unwrap();
    assert_eq!(totals.0, 16_000);
    assert_eq!(totals.0, totals.1);
}

//! Pruebas de integración del módulo de ventas.
//!
//! Cada prueba usa #[sqlx::test]: crea una base temporal, aplica ./migrations
//! y la elimina al terminar. No toca la base de desarrollo.
//! Requiere DATABASE_URL con un usuario que tenga permiso CREATEDB.
//!
//! Grupo A: caracterización (deben pasar hoy).
//! Grupo B: pruebas rojas (deben FALLAR hoy; demuestran un defecto del informe).

use std::sync::atomic::{AtomicU32, Ordering};

use chrono::NaiveDate;
use pos_lib::modules::sales::{
    dto::{CreateSaleDto, CreateSaleItemDto, CreateSalePaymentDto, SaleFilterDto},
    models::PaymentMethod,
    service,
};
use sqlx::PgPool;
use uuid::Uuid;

static SEQ: AtomicU32 = AtomicU32::new(1);

// ============================================================
// Helpers
// ============================================================

async fn seed_variant_full(
    pool: &PgPool,
    price: i64,
    stock: i32,
    allow_negative: bool,
    variant_active: bool,
    product_active: bool,
) -> i32 {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(format!("cat-{n}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name, is_active) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(category_id)
    .bind(format!("prod-{n}"))
    .bind(product_active)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants
             (product_id, attributes, price, stock, allow_negative, is_active)
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(product_id)
    .bind(price)
    .bind(stock)
    .bind(allow_negative)
    .bind(variant_active)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_variant(pool: &PgPool, price: i64, stock: i32) -> i32 {
    seed_variant_full(pool, price, stock, false, true, true).await
}

async fn open_cash(pool: &PgPool) -> i32 {
    sqlx::query_scalar("INSERT INTO cash_sessions (opening_amount) VALUES (0) RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(&sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stock_of(pool: &PgPool, variant_id: i32) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn item(variant_id: i32, quantity: i32) -> CreateSaleItemDto {
    CreateSaleItemDto {
        variant_id,
        quantity,
        discount: None,
    }
}

fn pay(method: PaymentMethod, amount: i64) -> CreateSalePaymentDto {
    CreateSalePaymentDto { method, amount }
}

fn new_sale(items: Vec<CreateSaleItemDto>, payments: Vec<CreateSalePaymentDto>) -> CreateSaleDto {
    CreateSaleDto {
        customer_id: None,
        items,
        payments,
        discount: None,
        cash_received: None,
        notes: None,
        courtesy_reason: None,
        created_by: Some("test".to_string()),
        idempotency_key: None,
    }
}

/// Una venta fallida no debe dejar NINGUNA fila ni cambiar el stock.
async fn assert_untouched(pool: &PgPool, variant_id: i32, expected_stock: i32) {
    for table in [
        "sales",
        "sale_items",
        "sale_payments",
        "inventory_movements",
        "cash_movements",
        "audit_events",
    ] {
        assert_eq!(
            count(pool, table).await,
            0,
            "la tabla {table} debe quedar vacía"
        );
    }
    assert_eq!(
        stock_of(pool, variant_id).await,
        expected_stock,
        "el stock no debe cambiar"
    );
}

// ============================================================
// GRUPO A: caracterización (deben pasar hoy)
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn a01_venta_en_efectivo_valida(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    let session_id = open_cash(&pool).await;

    let d = service::create_sale(
        &pool,
        new_sale(vec![item(v, 2)], vec![pay(PaymentMethod::Cash, 20_000)]),
    )
    .await
    .expect("la venta debe registrarse");

    assert_eq!(d.sale.subtotal, 20_000);
    assert_eq!(d.sale.total, 20_000);
    assert_eq!(d.items.len(), 1);
    assert_eq!(d.payments.len(), 1);
    assert_eq!(stock_of(&pool, v).await, 8);

    let (mtype, sale_id): (String, Option<i32>) =
        sqlx::query_as("SELECT movement_type::TEXT, sale_id FROM inventory_movements")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mtype, "sale");
    assert_eq!(sale_id, Some(d.sale.id));

    let (ctype, camount): (String, i64) =
        sqlx::query_as("SELECT movement_type::TEXT, amount FROM cash_movements")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ctype, "sale_in");
    assert_eq!(camount, 20_000);
    let (sale_session_id, movement_session_id): (i32, i32) = sqlx::query_as(
        "SELECT s.cash_session_id, cm.session_id
         FROM sales s JOIN cash_movements cm ON cm.sale_id = s.id
         WHERE s.id = $1",
    )
    .bind(d.sale.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (sale_session_id, movement_session_id),
        (session_id, session_id)
    );

    let audit: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events
         WHERE module = 'sales' AND action = 'create' AND outcome = 'success'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn a02_venta_con_tarjeta_exige_turno(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;

    let message = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await
    .expect_err("una venta con tarjeta también exige turno")
    .to_string();

    assert!(message.contains("turno de caja"), "{message}");
    assert_untouched(&pool, v, 10).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a03_efectivo_sin_turno_revierte_todo(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;

    let msg = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Cash, 10_000)]),
    )
    .await
    .expect_err("debe fallar sin turno abierto")
    .to_string();

    assert!(msg.contains("turno de caja"), "mensaje inesperado: {msg}");
    assert_untouched(&pool, v, 10).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a04_stock_insuficiente_revierte_todo(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 1).await;
    open_cash(&pool).await;

    let msg = service::create_sale(
        &pool,
        new_sale(vec![item(v, 2)], vec![pay(PaymentMethod::Card, 20_000)]),
    )
    .await
    .expect_err("debe fallar por stock")
    .to_string();

    assert!(
        msg.contains("Stock insuficiente"),
        "mensaje inesperado: {msg}"
    );
    assert_untouched(&pool, v, 1).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a05_allow_negative_permite_vender_sin_stock(pool: PgPool) {
    let v = seed_variant_full(&pool, 10_000, 1, true, true, true).await;
    open_cash(&pool).await;

    service::create_sale(
        &pool,
        new_sale(vec![item(v, 3)], vec![pay(PaymentMethod::Card, 30_000)]),
    )
    .await
    .expect("allow_negative debe permitir la venta");

    assert_eq!(stock_of(&pool, v).await, -2);
}

#[sqlx::test(migrations = "./migrations")]
async fn a06_varios_items_con_descuentos_cuadran(pool: PgPool) {
    let a = seed_variant(&pool, 10_000, 10).await;
    let b = seed_variant(&pool, 5_000, 10).await;
    open_cash(&pool).await;

    // bruto 25.000 - descuento de línea 1.000 = 24.000; - global 2.000 = 22.000
    let mut dto = new_sale(
        vec![
            CreateSaleItemDto {
                variant_id: a,
                quantity: 2,
                discount: Some(1_000),
            },
            item(b, 1),
        ],
        vec![pay(PaymentMethod::Card, 22_000)],
    );
    dto.discount = Some(2_000);

    let d = service::create_sale(&pool, dto)
        .await
        .expect("venta válida");

    let items_subtotal: i64 = d.items.iter().map(|i| i.subtotal).sum();
    let items_tax: i64 = d.items.iter().map(|i| i.tax).sum();
    let payments_total: i64 = d.payments.iter().map(|p| p.amount).sum();

    assert_eq!(d.sale.total, 22_000);
    assert_eq!(d.sale.discount, 2_000);
    assert_eq!(items_subtotal, d.sale.subtotal, "Σ ítems = subtotal");
    assert_eq!(
        d.sale.total,
        d.sale.subtotal - d.sale.discount,
        "total = subtotal - descuento"
    );
    assert_eq!(payments_total, d.sale.total, "Σ pagos = total");
    assert_eq!(
        items_tax, d.sale.tax,
        "Σ impuesto de ítems = impuesto de la venta"
    );
    assert_eq!(stock_of(&pool, a).await, 8);
    assert_eq!(stock_of(&pool, b).await, 9);
    assert_eq!(count(&pool, "inventory_movements").await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn a07_misma_variante_en_dos_lineas(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    service::create_sale(
        &pool,
        new_sale(
            vec![item(v, 2), item(v, 3)],
            vec![pay(PaymentMethod::Card, 50_000)],
        ),
    )
    .await
    .expect("venta válida");

    assert_eq!(stock_of(&pool, v).await, 0);
    let rows: Vec<(i32, i32)> =
        sqlx::query_as("SELECT stock_before, stock_after FROM inventory_movements ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows, vec![(5, 3), (3, 0)]);
}

#[sqlx::test(migrations = "./migrations")]
async fn a08_pagos_que_no_cuadran(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    let msg = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 9_000)]),
    )
    .await
    .expect_err("pagos distintos del total")
    .to_string();

    assert!(msg.contains("no coincide"), "mensaje inesperado: {msg}");
    assert_untouched(&pool, v, 10).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a09_descuento_mayor_al_bruto(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    let msg = service::create_sale(
        &pool,
        new_sale(
            vec![CreateSaleItemDto {
                variant_id: v,
                quantity: 1,
                discount: Some(10_001),
            }],
            vec![pay(PaymentMethod::Card, 1)],
        ),
    )
    .await
    .expect_err("descuento inválido")
    .to_string();

    assert!(msg.contains("descuento"), "mensaje inesperado: {msg}");
    assert_untouched(&pool, v, 10).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a10_cantidad_cero_se_rechaza(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    let msg = service::create_sale(
        &pool,
        new_sale(vec![item(v, 0)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await
    .expect_err("cantidad cero")
    .to_string();

    assert!(msg.contains("mayor a cero"), "mensaje inesperado: {msg}");
    assert_untouched(&pool, v, 10).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn a11_get_sale_devuelve_lo_creado(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;
    let created = service::create_sale(
        &pool,
        new_sale(vec![item(v, 2)], vec![pay(PaymentMethod::Transfer, 20_000)]),
    )
    .await
    .unwrap();

    let fetched = service::get_sale(&pool, created.sale.id).await.unwrap();
    assert_eq!(fetched.sale.total, created.sale.total);
    assert_eq!(fetched.items.len(), 1);
    assert_eq!(fetched.items[0].quantity, 2);
    assert_eq!(fetched.payments.len(), 1);

    assert!(service::get_sale(&pool, 999_999).await.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn a12_paginacion_basica(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;
    for _ in 0..3 {
        service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
        )
        .await
        .unwrap();
    }

    let r = service::list_sales(
        &pool,
        SaleFilterDto {
            page_size: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(r.data.len(), 2);
    assert_eq!(r.total, 3);
    assert_eq!(r.total_pages, 2);
}

/// Un fallo forzado en cada tabla escrita por la venta debe revertir todo.
#[sqlx::test(migrations = "./migrations")]
async fn a13_un_fallo_en_cualquier_paso_revierte_toda_la_venta(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    sqlx::query(
        "CREATE FUNCTION test_forced_failure() RETURNS trigger AS $$
         BEGIN RAISE EXCEPTION 'fallo forzado por la prueba'; END;
         $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .unwrap();

    for table in [
        "sale_items",
        "inventory_movements",
        "sale_payments",
        "cash_movements",
        "audit_events",
    ] {
        sqlx::query(&format!(
            "CREATE TRIGGER trg_test_forced_failure BEFORE INSERT ON {table}
             FOR EACH ROW EXECUTE FUNCTION test_forced_failure()"
        ))
        .execute(&pool)
        .await
        .unwrap();

        let result = service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Cash, 10_000)]),
        )
        .await;
        assert!(result.is_err(), "con fallo en {table} la venta debe fallar");
        assert_untouched(&pool, v, 10).await;

        sqlx::query(&format!("DROP TRIGGER trg_test_forced_failure ON {table}"))
            .execute(&pool)
            .await
            .unwrap();
    }
}

/// Dos ventas simultáneas de la última unidad: solo una puede completarse.
#[sqlx::test(migrations = "./migrations")]
async fn a14_dos_ventas_no_sobrevenden_la_ultima_unidad(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 1).await;
    open_cash(&pool).await;

    let (r1, r2) = tokio::join!(
        service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)])
        ),
        service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)])
        ),
    );

    let oks = [r1.is_ok(), r2.is_ok()].iter().filter(|ok| **ok).count();
    assert_eq!(oks, 1, "exactamente una venta debe completarse");
    assert_eq!(stock_of(&pool, v).await, 0);
    assert_eq!(count(&pool, "sales").await, 1);
    assert_eq!(count(&pool, "inventory_movements").await, 1);
}

// ============================================================
// GRUPO B: pruebas rojas (deben FALLAR hoy)
// ============================================================

/// A3. Contrato para la Etapa 2: mensaje "no está disponible".
#[sqlx::test(migrations = "./migrations")]
async fn b01_variante_inactiva_no_se_puede_vender(pool: PgPool) {
    let v = seed_variant_full(&pool, 10_000, 10, false, false, true).await;
    open_cash(&pool).await;

    let result = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await;

    let msg = result
        .expect_err("una variante inactiva no debe venderse")
        .to_string();
    assert!(
        msg.contains("no está disponible"),
        "mensaje inesperado: {msg}"
    );
    assert_untouched(&pool, v, 10).await;
}

/// A3. Variante activa pero producto inactivo.
#[sqlx::test(migrations = "./migrations")]
async fn b02_producto_inactivo_no_se_puede_vender(pool: PgPool) {
    let v = seed_variant_full(&pool, 10_000, 10, false, true, false).await;
    open_cash(&pool).await;

    let result = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await;

    let msg = result
        .expect_err("un producto inactivo no debe venderse")
        .to_string();
    assert!(
        msg.contains("no está disponible"),
        "mensaje inesperado: {msg}"
    );
    assert_untouched(&pool, v, 10).await;
}

/// N3. Una variante inexistente no debe mostrar el error crudo de la base.
#[sqlx::test(migrations = "./migrations")]
async fn b03_variante_inexistente_da_mensaje_de_negocio(pool: PgPool) {
    open_cash(&pool).await;
    let msg = service::create_sale(
        &pool,
        new_sale(
            vec![item(999_999, 1)],
            vec![pay(PaymentMethod::Card, 10_000)],
        ),
    )
    .await
    .expect_err("variante inexistente")
    .to_string();

    assert!(
        !msg.contains("Error de base de datos"),
        "se filtró el error técnico: {msg}"
    );
    assert!(msg.contains("no existe"), "mensaje inesperado: {msg}");
}

/// M4 ampliado: dos ventas simultáneas del MISMO producto, mismo orden.
/// Hipótesis: el FOR KEY SHARE de la FK en sale_items choca con el FOR UPDATE.
#[sqlx::test(migrations = "./migrations")]
async fn b04_mismo_producto_concurrente_no_se_bloquea(pool: PgPool) {
    let a = seed_variant(&pool, 10_000, 1_000).await;
    open_cash(&pool).await;

    for round in 0..20 {
        let (r1, r2) = tokio::join!(
            service::create_sale(
                &pool,
                new_sale(vec![item(a, 1)], vec![pay(PaymentMethod::Card, 10_000)])
            ),
            service::create_sale(
                &pool,
                new_sale(vec![item(a, 1)], vec![pay(PaymentMethod::Card, 10_000)])
            ),
        );
        assert!(
            r1.is_ok(),
            "ronda {round}, venta 1: {:?}",
            r1.as_ref().err()
        );
        assert!(
            r2.is_ok(),
            "ronda {round}, venta 2: {:?}",
            r2.as_ref().err()
        );
    }
}

/// M4: orden cruzado de variantes.
#[sqlx::test(migrations = "./migrations")]
async fn b05_orden_cruzado_de_variantes_no_se_bloquea(pool: PgPool) {
    let a = seed_variant(&pool, 10_000, 1_000).await;
    let b = seed_variant(&pool, 5_000, 1_000).await;
    open_cash(&pool).await;

    for round in 0..20 {
        let (r1, r2) = tokio::join!(
            service::create_sale(
                &pool,
                new_sale(
                    vec![item(a, 1), item(b, 1)],
                    vec![pay(PaymentMethod::Card, 15_000)]
                )
            ),
            service::create_sale(
                &pool,
                new_sale(
                    vec![item(b, 1), item(a, 1)],
                    vec![pay(PaymentMethod::Card, 15_000)]
                )
            ),
        );
        assert!(
            r1.is_ok(),
            "ronda {round}, venta 1: {:?}",
            r1.as_ref().err()
        );
        assert!(
            r2.is_ok(),
            "ronda {round}, venta 2: {:?}",
            r2.as_ref().err()
        );
    }
}

/// M6. El service debe normalizar page_size igual que el repository (1..=100).
#[sqlx::test(migrations = "./migrations")]
async fn b06_page_size_se_normaliza_en_la_respuesta(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;
    for _ in 0..3 {
        service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
        )
        .await
        .unwrap();
    }

    let r = service::list_sales(
        &pool,
        SaleFilterDto {
            page_size: Some(0),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(r.page_size, 1);
    assert_eq!(r.data.len(), 1);
    assert_eq!(r.total_pages, 3);

    let r = service::list_sales(
        &pool,
        SaleFilterDto {
            page_size: Some(500),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(r.page_size, 100);
}

/// N8. Una venta a las 10:30 p. m. de Bogotá del día 8 debe aparecer al filtrar el día 8.
#[sqlx::test(migrations = "./migrations")]
async fn b07_filtro_de_fecha_respeta_la_zona_de_ajustes(pool: PgPool) {
    sqlx::query(
        "INSERT INTO sales (subtotal, total, created_at)
         VALUES (1000, 1000, '2026-10-08 22:30:00-05'::timestamptz)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let day = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
    let r = service::list_sales(
        &pool,
        SaleFilterDto {
            date_from: Some(day),
            date_to: Some(day),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(
        r.total, 1,
        "la venta de las 22:30 (Bogotá) debe caer en el día 8"
    );
}

// ============================================================
// GRUPO C: idempotencia
// ============================================================

fn keyed(
    key: Uuid,
    items: Vec<CreateSaleItemDto>,
    payments: Vec<CreateSalePaymentDto>,
) -> CreateSaleDto {
    let mut dto = new_sale(items, payments);
    dto.idempotency_key = Some(key);
    dto
}

#[sqlx::test(migrations = "./migrations")]
async fn c01_reintento_secuencial_devuelve_la_misma_venta(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;
    let key = Uuid::new_v4();
    let make = || {
        keyed(
            key,
            vec![item(v, 2)],
            vec![pay(PaymentMethod::Cash, 20_000)],
        )
    };

    let first = service::create_sale(&pool, make()).await.unwrap();
    let second = service::create_sale(&pool, make()).await.unwrap();

    assert_eq!(first.sale.id, second.sale.id);
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.payments.len(), 1);
    assert_eq!(count(&pool, "sales").await, 1);
    assert_eq!(count(&pool, "inventory_movements").await, 1);
    assert_eq!(count(&pool, "cash_movements").await, 1);
    assert_eq!(
        stock_of(&pool, v).await,
        8,
        "el stock se descuenta una sola vez"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn c02_reintentos_simultaneos_crean_una_sola_venta(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;
    let key = Uuid::new_v4();
    let make = || {
        keyed(
            key,
            vec![item(v, 1)],
            vec![pay(PaymentMethod::Card, 10_000)],
        )
    };

    let (r1, r2, r3) = tokio::join!(
        service::create_sale(&pool, make()),
        service::create_sale(&pool, make()),
        service::create_sale(&pool, make()),
    );

    let a = r1.expect("intento 1");
    let b = r2.expect("intento 2");
    let c = r3.expect("intento 3");
    assert_eq!(a.sale.id, b.sale.id);
    assert_eq!(b.sale.id, c.sale.id);
    assert_eq!(count(&pool, "sales").await, 1);
    assert_eq!(stock_of(&pool, v).await, 9);
}

#[sqlx::test(migrations = "./migrations")]
async fn c03_claves_distintas_crean_ventas_distintas(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    for _ in 0..2 {
        service::create_sale(
            &pool,
            keyed(
                Uuid::new_v4(),
                vec![item(v, 1)],
                vec![pay(PaymentMethod::Card, 10_000)],
            ),
        )
        .await
        .unwrap();
    }

    assert_eq!(count(&pool, "sales").await, 2);
    assert_eq!(stock_of(&pool, v).await, 8);
}

#[sqlx::test(migrations = "./migrations")]
async fn c04_sin_clave_no_hay_deduplicacion(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    open_cash(&pool).await;

    for _ in 0..2 {
        service::create_sale(
            &pool,
            new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
        )
        .await
        .unwrap();
    }

    assert_eq!(count(&pool, "sales").await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn c05_un_fallo_no_consume_la_clave(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 1).await;
    open_cash(&pool).await;
    let key = Uuid::new_v4();

    let failed = service::create_sale(
        &pool,
        keyed(
            key,
            vec![item(v, 2)],
            vec![pay(PaymentMethod::Card, 20_000)],
        ),
    )
    .await;
    assert!(failed.is_err());
    assert_untouched(&pool, v, 1).await;

    sqlx::query("UPDATE product_variants SET stock = 5 WHERE id = $1")
        .bind(v)
        .execute(&pool)
        .await
        .unwrap();

    let ok = service::create_sale(
        &pool,
        keyed(
            key,
            vec![item(v, 2)],
            vec![pay(PaymentMethod::Card, 20_000)],
        ),
    )
    .await
    .expect("la clave no debió quedar consumida por el fallo");

    assert_eq!(count(&pool, "sales").await, 1);
    assert_eq!(stock_of(&pool, v).await, 3);
    assert!(ok.sale.id > 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn c06_reintento_idempotente_tras_cerrar_turno_devuelve_venta_original(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    let session_id = open_cash(&pool).await;
    let key = Uuid::new_v4();
    let make = || {
        keyed(
            key,
            vec![item(v, 1)],
            vec![pay(PaymentMethod::Card, 10_000)],
        )
    };

    let first = service::create_sale(&pool, make()).await.unwrap();
    sqlx::query("UPDATE cash_sessions SET status = 'closed', closed_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();

    let retry = service::create_sale(&pool, make())
        .await
        .expect("reintentar una venta existente no debe exigir una caja abierta");

    assert_eq!(retry.sale.id, first.sale.id);
    assert_eq!(count(&pool, "sales").await, 1);
    assert_eq!(stock_of(&pool, v).await, 9);
}

#[sqlx::test(migrations = "./migrations")]
async fn a15_venta_con_tarjeta_guarda_turno_sin_movimiento_de_caja(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;
    let session_id = open_cash(&pool).await;

    let sale = service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await
    .unwrap();
    let linked_session_id: i32 =
        sqlx::query_scalar("SELECT cash_session_id FROM sales WHERE id = $1")
            .bind(sale.sale.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(linked_session_id, session_id);
    assert_eq!(count(&pool, "cash_movements").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn a16_venta_de_cortesia_guarda_motivo_y_no_registra_pago(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("Apoyo a cliente".to_string());

    let created = service::create_sale(&pool, dto)
        .await
        .expect("la cortesía debe registrarse sin pago");

    assert_eq!(created.sale.total, 0);
    assert_eq!(created.sale.discount, created.sale.subtotal);
    assert_eq!(
        created.sale.courtesy_reason.as_deref(),
        Some("Apoyo a cliente")
    );
    let saved_session_id: i32 =
        sqlx::query_scalar("SELECT cash_session_id FROM sales WHERE id = $1")
            .bind(created.sale.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(saved_session_id, session_id);
    assert!(created.payments.is_empty());
    assert_eq!(count(&pool, "sale_payments").await, 0);
    assert_eq!(stock_of(&pool, variant_id).await, 4);
    assert_eq!(count(&pool, "cash_movements").await, 0);

    let fetched = service::get_sale(&pool, created.sale.id)
        .await
        .expect("el detalle debe incluir el motivo");
    assert_eq!(
        fetched.sale.courtesy_reason.as_deref(),
        Some("Apoyo a cliente")
    );
    let listed = service::list_sales(&pool, SaleFilterDto::default())
        .await
        .unwrap();
    assert_eq!(
        listed.data[0].courtesy_reason.as_deref(),
        Some("Apoyo a cliente")
    );

    let metadata: serde_json::Value =
        sqlx::query_scalar("SELECT metadata FROM audit_events WHERE action = 'create'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata["courtesy"], true);
    assert_eq!(metadata["courtesy_reason"], "Apoyo a cliente");
}

#[sqlx::test(migrations = "./migrations")]
async fn a17_total_cero_sin_motivo_de_cortesia_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("una venta total cero requiere motivo")
        .to_string();

    assert!(message.contains("cortesía"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d01_venta_con_valor_y_motivo_sin_pago_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.courtesy_reason = Some("Motivo recibido".to_string());
    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("una venta con valor requiere pago")
        .to_string();
    assert!(message.contains("al menos un pago"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d02_venta_con_valor_y_motivo_se_guarda_como_venta_normal(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(
        vec![item(variant_id, 1)],
        vec![pay(PaymentMethod::Card, 10_000)],
    );
    dto.courtesy_reason = Some("Motivo que debe descartarse".to_string());

    let created = service::create_sale(&pool, dto)
        .await
        .expect("debe registrarse como venta normal");
    let saved_reason: Option<String> =
        sqlx::query_scalar("SELECT courtesy_reason FROM sales WHERE id = $1")
            .bind(created.sale.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(created.sale.total, 10_000);
    assert_eq!(saved_reason, None);
    assert_eq!(created.sale.courtesy_reason, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn d03_total_cero_con_pago_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(
        vec![item(variant_id, 1)],
        vec![pay(PaymentMethod::Card, 1)],
    );
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("Apoyo".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("una cortesía no debe incluir pagos")
        .to_string();
    assert!(message.contains("no puede incluir pagos"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d04_motivo_de_cortesia_solo_espacios_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("   ".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("el motivo no puede estar vacío")
        .to_string();
    assert!(message.to_lowercase().contains("motivo"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d05_motivo_de_200_caracteres_se_acepta(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let reason = "a".repeat(200);
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some(reason.clone());

    let created = service::create_sale(&pool, dto)
        .await
        .expect("un motivo de 200 caracteres debe aceptarse");
    assert_eq!(created.sale.courtesy_reason.as_deref(), Some(reason.as_str()));
}

#[sqlx::test(migrations = "./migrations")]
async fn d06_motivo_de_201_caracteres_se_rechaza_sin_evento_de_fallo(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("a".repeat(201));

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("un motivo de 201 caracteres debe rechazarse")
        .to_string();
    assert!(message.contains("200 caracteres"), "{message}");
    let failure_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE outcome::TEXT = 'failure'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failure_events, 0);
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d07_motivo_con_acentos_se_guarda_sin_espacios_laterales(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("  Apoyo a José  ".to_string());

    let created = service::create_sale(&pool, dto)
        .await
        .expect("el motivo con acentos debe aceptarse");
    assert_eq!(
        created.sale.courtesy_reason.as_deref(),
        Some("Apoyo a José")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d08_bruto_cero_se_rechaza_como_cortesia(pool: PgPool) {
    let variant_id = seed_variant(&pool, 0, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.courtesy_reason = Some("Apoyo".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("una cortesía requiere importe bruto mayor a cero")
        .to_string();
    assert!(message.contains("importe bruto"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d09_descuento_de_linea_total_permite_cortesia(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut line = item(variant_id, 1);
    line.discount = Some(10_000);
    let mut dto = new_sale(vec![line], vec![]);
    dto.courtesy_reason = Some("Apoyo".to_string());

    let created = service::create_sale(&pool, dto)
        .await
        .expect("el descuento total de línea debe permitir cortesía");
    assert_eq!(created.sale.total, 0);
    assert_eq!(created.sale.discount, 0);
    assert_eq!(created.sale.courtesy_reason.as_deref(), Some("Apoyo"));
}

#[sqlx::test(migrations = "./migrations")]
async fn d10_descuento_parcial_sin_pagos_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(1_000);
    dto.courtesy_reason = Some("No debe convertir la venta".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("el descuento parcial deja saldo y requiere pago")
        .to_string();
    assert!(message.contains("al menos un pago"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d11_cortesia_sin_turno_abierto_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let mut dto = new_sale(vec![item(variant_id, 1)], vec![]);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("Apoyo".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("la cortesía también requiere turno abierto")
        .to_string();
    assert!(message.contains("turno de caja"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn d12_reintento_idempotente_de_cortesia_devuelve_la_misma_venta(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let key = Uuid::new_v4();
    let make = || {
        let mut dto = keyed(key, vec![item(variant_id, 1)], vec![]);
        dto.discount = Some(10_000);
        dto.courtesy_reason = Some("Apoyo".to_string());
        dto
    };

    let first = service::create_sale(&pool, make()).await.unwrap();
    let retry = service::create_sale(&pool, make()).await.unwrap();
    assert_eq!(first.sale.id, retry.sale.id);
    assert_eq!(count(&pool, "sales").await, 1);
}

// ============================================================
// GRUPO E: efectivo recibido y cambio
// ============================================================

fn sale_with_cash_received(
    items: Vec<CreateSaleItemDto>,
    payments: Vec<CreateSalePaymentDto>,
    cash_received: i64,
) -> CreateSaleDto {
    let mut dto = new_sale(items, payments);
    dto.cash_received = Some(cash_received);
    dto
}

#[sqlx::test(migrations = "./migrations")]
async fn e01_efectivo_con_vuelto_guarda_neto_y_turno(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_cash(&pool).await;

    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            20_000,
        ),
    )
    .await
    .unwrap();

    assert_eq!(sale.sale.cash_received, Some(20_000));
    assert_eq!(sale.sale.change_given, Some(10_000));
    let payments_total: i64 =
        sqlx::query_scalar("SELECT SUM(amount) FROM sale_payments WHERE sale_id = $1")
            .bind(sale.sale.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(payments_total, 10_000);
    let (movement_type, movement_amount, saved_session_id): (String, i64, i32) =
        sqlx::query_as(
            "SELECT cm.movement_type::TEXT, cm.amount, s.cash_session_id
             FROM cash_movements cm JOIN sales s ON s.id = cm.sale_id
             WHERE cm.sale_id = $1",
        )
        .bind(sale.sale.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(movement_type, "sale_in");
    assert_eq!(movement_amount, 10_000);
    assert_eq!(saved_session_id, session_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn e02_pago_mixto_calcula_vuelto_sobre_neto_en_efectivo(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![
                pay(PaymentMethod::Card, 4_000),
                pay(PaymentMethod::Cash, 6_000),
            ],
            10_000,
        ),
    )
    .await
    .unwrap();

    assert_eq!(sale.sale.cash_received, Some(10_000));
    assert_eq!(sale.sale.change_given, Some(4_000));
    let (movement_type, movement_amount): (String, i64) =
        sqlx::query_as("SELECT movement_type::TEXT, amount FROM cash_movements")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(movement_type, "sale_in");
    assert_eq!(movement_amount, 6_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn e03_recibido_menor_que_efectivo_revierte_todo(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    for received in [5_000, 0] {
        let message = service::create_sale(
            &pool,
            sale_with_cash_received(
                vec![item(variant_id, 1)],
                vec![pay(PaymentMethod::Cash, 10_000)],
                received,
            ),
        )
        .await
        .expect_err("el efectivo recibido no puede ser menor")
        .to_string();
        assert!(message.contains("no puede ser menor"), "{message}");
        assert_untouched(&pool, variant_id, 5).await;
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn e04_recibido_sin_pago_en_efectivo_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let message = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Card, 10_000)],
            10_000,
        ),
    )
    .await
    .expect_err("el recibido requiere efectivo en los pagos")
    .to_string();

    assert!(
        message.contains("requiere una venta con pago en efectivo"),
        "{message}"
    );
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn e05_precision_de_efectivo_recibido_sigue_la_moneda(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let message = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            10_050,
        ),
    )
    .await
    .expect_err("con cero decimales se rechazan fracciones")
    .to_string();
    assert!(message.contains("fracciones"), "{message}");
    assert_untouched(&pool, variant_id, 5).await;

    sqlx::query("UPDATE app_settings SET currency = 'USD', currency_decimals = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            10_050,
        ),
    )
    .await
    .expect("con dos decimales se aceptan centavos");
    assert_eq!(sale.sale.cash_received, Some(10_050));
    assert_eq!(sale.sale.change_given, Some(50));
}

#[sqlx::test(migrations = "./migrations")]
async fn e06_cortesia_no_acepta_efectivo_recibido(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = sale_with_cash_received(vec![item(variant_id, 1)], vec![], 10_000);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("Apoyo".to_string());

    let message = service::create_sale(&pool, dto)
        .await
        .expect_err("una cortesía no puede recibir efectivo")
        .to_string();

    assert!(
        message.contains("requiere una venta con pago en efectivo"),
        "{message}"
    );
    assert_untouched(&pool, variant_id, 5).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn e07_sin_recibido_los_dos_campos_se_guardan_null(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let sale = service::create_sale(
        &pool,
        new_sale(vec![item(variant_id, 1)], vec![pay(PaymentMethod::Cash, 10_000)]),
    )
    .await
    .unwrap();

    assert_eq!(sale.sale.cash_received, None);
    assert_eq!(sale.sale.change_given, None);
    let (cash_received, change_given): (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT cash_received, change_given FROM sales WHERE id = $1")
            .bind(sale.sale.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((cash_received, change_given), (None, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn e08_recibido_exacto_guarda_cambio_cero(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;

    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            10_000,
        ),
    )
    .await
    .unwrap();

    assert_eq!(sale.sale.cash_received, Some(10_000));
    assert_eq!(sale.sale.change_given, Some(0));
}

#[sqlx::test(migrations = "./migrations")]
async fn e09_check_de_bd_rechaza_recibido_inconsistente(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let valid = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            20_000,
        ),
    )
    .await
    .unwrap();

    let without_change = sqlx::query(
        "UPDATE sales SET cash_received = 20_000, change_given = NULL WHERE id = $1",
    )
    .bind(valid.sale.id)
    .execute(&pool)
    .await;
    assert!(without_change.is_err());

    let change_not_below_received = sqlx::query(
        "UPDATE sales SET cash_received = 10_000, change_given = 10_000 WHERE id = $1",
    )
    .bind(valid.sale.id)
    .execute(&pool)
    .await;
    assert!(change_not_below_received.is_err());

    let mut courtesy = new_sale(vec![item(variant_id, 1)], vec![]);
    courtesy.discount = Some(10_000);
    courtesy.courtesy_reason = Some("Apoyo".to_string());
    let free_sale = service::create_sale(&pool, courtesy).await.unwrap();
    let received_on_zero_total = sqlx::query(
        "UPDATE sales SET cash_received = 1, change_given = 0 WHERE id = $1",
    )
    .bind(free_sale.sale.id)
    .execute(&pool)
    .await;
    assert!(received_on_zero_total.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn e10_idempotencia_con_recibido_retorna_los_mismos_valores(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let key = Uuid::new_v4();
    let make = || {
        let mut dto = keyed(
            key,
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
        );
        dto.cash_received = Some(20_000);
        dto
    };

    let first = service::create_sale(&pool, make()).await.unwrap();
    let retry = service::create_sale(&pool, make()).await.unwrap();

    assert_eq!(first.sale.id, retry.sale.id);
    assert_eq!(first.sale.cash_received, Some(20_000));
    assert_eq!(retry.sale.cash_received, Some(20_000));
    assert_eq!(first.sale.change_given, Some(10_000));
    assert_eq!(retry.sale.change_given, Some(10_000));
}

#[sqlx::test(migrations = "./migrations")]
async fn e11_auditoria_incluye_recibido_y_cambio_en_metadata(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            20_000,
        ),
    )
    .await
    .unwrap();

    let metadata: serde_json::Value =
        sqlx::query_scalar("SELECT metadata FROM audit_events WHERE action = 'create'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata["cash_received"], 20_000);
    assert_eq!(metadata["change_given"], 10_000);
    assert_eq!(sale.sale.cash_received, Some(20_000));
}

#[sqlx::test(migrations = "./migrations")]
async fn e12_get_y_list_sales_incluyen_recibido_y_cambio(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale = service::create_sale(
        &pool,
        sale_with_cash_received(
            vec![item(variant_id, 1)],
            vec![pay(PaymentMethod::Cash, 10_000)],
            20_000,
        ),
    )
    .await
    .unwrap();

    let fetched = service::get_sale(&pool, sale.sale.id).await.unwrap();
    assert_eq!(fetched.sale.cash_received, Some(20_000));
    assert_eq!(fetched.sale.change_given, Some(10_000));

    let listed = service::list_sales(&pool, SaleFilterDto::default())
        .await
        .unwrap();
    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].cash_received, Some(20_000));
    assert_eq!(listed.data[0].change_given, Some(10_000));
}

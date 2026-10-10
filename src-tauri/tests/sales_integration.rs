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

async fn open_cash(pool: &PgPool) {
    sqlx::query("INSERT INTO cash_sessions (opening_amount) VALUES (0)")
        .execute(pool)
        .await
        .unwrap();
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
        notes: None,
        created_by: Some("test".to_string()),
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
    open_cash(&pool).await;

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

    let audit: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events
         WHERE module = 'sales' AND action = 'create' AND outcome = 'success'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, 1);
}

/// Documenta el comportamiento ACTUAL (decisión M3 pendiente): una venta sin
/// efectivo se registra aunque no haya turno de caja abierto.
#[sqlx::test(migrations = "./migrations")]
async fn a02_venta_con_tarjeta_no_requiere_turno_hoy(pool: PgPool) {
    let v = seed_variant(&pool, 10_000, 10).await;

    service::create_sale(
        &pool,
        new_sale(vec![item(v, 1)], vec![pay(PaymentMethod::Card, 10_000)]),
    )
    .await
    .expect("hoy la venta con tarjeta se registra sin turno");

    assert_eq!(count(&pool, "cash_movements").await, 0);
    assert_eq!(stock_of(&pool, v).await, 9);
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

use pos_lib::modules::{
    cash::{
        dto::CloseSessionDto,
        models::CashSessionReport,
        repository as cash_repository, service as cash_service,
    },
    sales::{
        dto::{CreateSaleDto, CreateSaleItemDto, CreateSalePaymentDto, VoidSaleDto},
        models::{PaymentMethod, SaleStatus},
        service as sales_service,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_variant(pool: &PgPool, price: i64, stock: i32) -> i32 {
    let suffix = Uuid::new_v4().simple().to_string();
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(format!("cash-report-category-{suffix}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name, is_active) VALUES ($1, $2, TRUE) RETURNING id",
    )
    .bind(category_id)
    .bind(format!("cash-report-product-{suffix}"))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price, stock, is_active) \
         VALUES ($1, '{}'::jsonb, $2, $3, TRUE) RETURNING id",
    )
    .bind(product_id)
    .bind(price)
    .bind(stock)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn open_session(pool: &PgPool, opening_amount: i64) -> i32 {
    sqlx::query_scalar("INSERT INTO cash_sessions (opening_amount) VALUES ($1) RETURNING id")
        .bind(opening_amount)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn close_session(pool: &PgPool, id: i32, counted_amount: i64) -> Result<(), String> {
    cash_service::close_session(
        pool,
        id,
        CloseSessionDto {
            counted_amount,
            closing_notes: None,
            closed_by: Some("cash-report-test".to_string()),
        },
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn sale(
    variant_id: i32,
    payments: Vec<CreateSalePaymentDto>,
    discount: Option<i64>,
    courtesy_reason: Option<&str>,
    cash_received: Option<i64>,
) -> CreateSaleDto {
    CreateSaleDto {
        customer_id: None,
        items: vec![CreateSaleItemDto {
            variant_id,
            quantity: 1,
            discount: None,
        }],
        payments,
        discount,
        cash_received,
        notes: None,
        courtesy_reason: courtesy_reason.map(str::to_string),
        created_by: Some("cash-report-test".to_string()),
        idempotency_key: None,
    }
}

fn payment(method: PaymentMethod, amount: i64) -> CreateSalePaymentDto {
    CreateSalePaymentDto { method, amount }
}

async fn stored_report(pool: &PgPool, id: i32) -> CashSessionReport {
    cash_repository::get_stored_report(pool, id)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn r01_informe_x_resume_efectivo_ventas_cortesia_y_esperado(pool: PgPool) {
    let cash_variant = seed_variant(&pool, 10_000, 10).await;
    let card_variant = seed_variant(&pool, 4_000, 10).await;
    let mixed_variant = seed_variant(&pool, 10_000, 10).await;
    let courtesy_variant = seed_variant(&pool, 10_000, 10).await;
    let session_id = open_session(&pool, 500).await;

    let cash_sale = sale(
        cash_variant,
        vec![payment(PaymentMethod::Cash, 10_000)],
        None,
        None,
        Some(20_000),
    );
    sales_service::create_sale(&pool, cash_sale).await.unwrap();
    sales_service::create_sale(
        &pool,
        sale(
            card_variant,
            vec![payment(PaymentMethod::Card, 4_000)],
            None,
            None,
            None,
        ),
    )
    .await
    .unwrap();
    sales_service::create_sale(
        &pool,
        sale(
            mixed_variant,
            vec![
                payment(PaymentMethod::Card, 4_000),
                payment(PaymentMethod::Cash, 6_000),
            ],
            None,
            None,
            Some(10_000),
        ),
    )
    .await
    .unwrap();
    sales_service::create_sale(
        &pool,
        sale(courtesy_variant, vec![], Some(10_000), Some("Apoyo"), None),
    )
    .await
    .unwrap();

    let report = cash_service::get_session_report(&pool, session_id)
        .await
        .unwrap();
    assert_eq!(report.kind, "X");
    assert_eq!(report.cash.cash_sales, 16_000);
    assert_eq!(report.cash.expected_amount, 16_500);
    assert_eq!(report.sales.count, 4);
    assert_eq!(report.sales.gross_total, 34_000);
    assert_eq!(report.sales.discount_total, 10_000);
    assert_eq!(report.sales.net_total, 24_000);
    assert_eq!(report.sales.by_method.cash, 16_000);
    assert_eq!(report.sales.by_method.card, 8_000);
    assert_eq!(report.sales.by_method.transfer, 0);
    assert_eq!(report.sales.courtesy_count, 1);
    assert_eq!(report.sales.courtesy_gross, 10_000);
    let status: String = sqlx::query_scalar("SELECT status FROM cash_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open");
    assert!(cash_repository::get_stored_report(&pool, session_id)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn r02_esperado_del_informe_z_coincide_con_el_cierre(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_session(&pool, 2_000).await;
    sales_service::create_sale(
        &pool,
        sale(
            variant_id,
            vec![payment(PaymentMethod::Cash, 10_000)],
            None,
            None,
            None,
        ),
    )
    .await
    .unwrap();

    close_session(&pool, session_id, 12_000).await.unwrap();
    let report = stored_report(&pool, session_id).await;
    let expected: i64 = sqlx::query_scalar(
        "SELECT expected_amount FROM cash_sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(report.kind, "Z");
    assert_eq!(report.cash.expected_amount, expected);
}

#[sqlx::test(migrations = "./migrations")]
async fn r03_z_se_conserva_y_anulacion_se_asigna_al_turno_de_realizacion(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let first_session = open_session(&pool, 0).await;
    let created = sales_service::create_sale(
        &pool,
        sale(
            variant_id,
            vec![payment(PaymentMethod::Card, 10_000)],
            None,
            None,
            None,
        ),
    )
    .await
    .unwrap();
    close_session(&pool, first_session, 0).await.unwrap();
    let first_before = stored_report(&pool, first_session).await;

    let second_session = open_session(&pool, 1_000).await;
    sales_service::void_sale(
        &pool,
        created.sale.id,
        VoidSaleDto {
            target_status: SaleStatus::Cancelled,
            reason: "Prueba de atribución".to_string(),
            performed_by: Some("cash-report-test".to_string()),
        },
    )
    .await
    .unwrap();

    let first_after = cash_service::get_session_report(&pool, first_session)
        .await
        .unwrap();
    let second_report = cash_service::get_session_report(&pool, second_session)
        .await
        .unwrap();
    assert_eq!(first_after.voids.cancelled_count, first_before.voids.cancelled_count);
    assert_eq!(first_after.voids.cancelled_total, first_before.voids.cancelled_total);
    assert_eq!(second_report.kind, "X");
    assert_eq!(second_report.voids.cancelled_count, 1);
    assert_eq!(second_report.voids.cancelled_total, 10_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn r04_z_guarda_diferencias_positivas_negativas_y_cero(pool: PgPool) {
    for (opening, counted, expected_difference) in [
        (100_000, 110_000, 10_000),
        (100_000, 90_000, -10_000),
        (100_000, 100_000, 0),
    ] {
        let session_id = open_session(&pool, opening).await;
        close_session(&pool, session_id, counted).await.unwrap();
        let report = stored_report(&pool, session_id).await;
        assert_eq!(report.cash.counted_amount, Some(counted));
        assert_eq!(report.cash.expected_amount, opening);
        assert_eq!(report.cash.difference, Some(expected_difference));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r05_venta_sin_turno_no_entra_en_informe(pool: PgPool) {
    let session_id = open_session(&pool, 0).await;
    sqlx::query(
        "INSERT INTO sales (subtotal, tax, discount, total, status, created_by, cash_session_id) \
         VALUES (5000, 0, 0, 5000, 'completed', 'legacy-test', NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let report = cash_service::get_session_report(&pool, session_id)
        .await
        .unwrap();
    assert_eq!(report.sales.count, 0);
    assert_eq!(report.sales.gross_total, 0);
    assert_eq!(report.sales.net_total, 0);
    assert_eq!(report.sales.by_method.cash, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn r06_turno_cerrado_sin_z_reporta_que_no_hay_informe(pool: PgPool) {
    let session_id = open_session(&pool, 0).await;
    sqlx::query(
        "UPDATE cash_sessions SET status = 'closed', closed_at = NOW() WHERE id = $1",
    )
    .bind(session_id)
    .execute(&pool)
    .await
    .unwrap();

    let error = cash_service::get_session_report(&pool, session_id)
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "Este turno se cerró antes de existir el informe Z; no hay un informe guardado."
    );
}

async fn make_report_insert_fail(pool: &PgPool) {
    sqlx::query(
        r#"CREATE FUNCTION fail_report_insert() RETURNS trigger AS $$
           BEGIN RAISE EXCEPTION 'cash report insert failed'; END;
           $$ LANGUAGE plpgsql"#,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER trg_fail_cash_report_insert BEFORE INSERT ON cash_session_reports \
         FOR EACH ROW EXECUTE FUNCTION fail_report_insert()",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn make_audit_insert_fail(pool: &PgPool) {
    sqlx::query(
        r#"CREATE FUNCTION fail_cash_audit_insert() RETURNS trigger AS $$
           BEGIN RAISE EXCEPTION 'cash audit insert failed'; END;
           $$ LANGUAGE plpgsql"#,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER trg_fail_cash_audit_insert BEFORE INSERT ON audit_events \
         FOR EACH ROW EXECUTE FUNCTION fail_cash_audit_insert()",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn r07_fallo_de_insercion_de_z_o_auditoria_revierte_el_cierre(pool: PgPool) {
    let session_id = open_session(&pool, 5_000).await;
    make_report_insert_fail(&pool).await;
    assert!(close_session(&pool, session_id, 5_000).await.is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM cash_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open");
    assert!(cash_repository::get_stored_report(&pool, session_id)
        .await
        .unwrap()
        .is_none());

    sqlx::query("DROP TRIGGER trg_fail_cash_report_insert ON cash_session_reports")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION fail_report_insert()")
        .execute(&pool)
        .await
        .unwrap();
    make_audit_insert_fail(&pool).await;
    assert!(close_session(&pool, session_id, 5_000).await.is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM cash_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open");
    assert!(cash_repository::get_stored_report(&pool, session_id)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn r08_cierre_concurrente_con_venta_conserva_un_estado_coherente(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_session(&pool, 1_000).await;
    let (close_result, sale_result) = tokio::join!(
        cash_service::close_session(
            &pool,
            session_id,
            CloseSessionDto {
                counted_amount: 1_000,
                closing_notes: None,
                closed_by: Some("cash-report-test".to_string()),
            },
        ),
        sales_service::create_sale(
            &pool,
            sale(
                variant_id,
                vec![payment(PaymentMethod::Cash, 10_000)],
                None,
                None,
                None,
            ),
        )
    );
    close_result.unwrap();
    let report = stored_report(&pool, session_id).await;

    match sale_result {
        Ok(_) => {
            assert_eq!(report.sales.by_method.cash, 10_000);
            assert_eq!(report.cash.cash_sales, 10_000);
            assert_eq!(report.cash.expected_amount, 11_000);
        }
        Err(error) => {
            assert!(error.to_string().contains("No hay un turno de caja abierto"));
            assert_eq!(report.sales.count, 0);
            assert_eq!(report.cash.cash_sales, 0);
            assert_eq!(report.cash.expected_amount, 1_000);
            let stock: i32 =
                sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
                    .bind(variant_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(stock, 5);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r09_devolucion_en_efectivo_informa_sale_out_y_reduce_esperado(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_session(&pool, 5_000).await;
    let created = sales_service::create_sale(
        &pool,
        sale(
            variant_id,
            vec![payment(PaymentMethod::Cash, 10_000)],
            None,
            None,
            Some(20_000),
        ),
    )
    .await
    .unwrap();
    sales_service::void_sale(
        &pool,
        created.sale.id,
        VoidSaleDto {
            target_status: SaleStatus::Refunded,
            reason: "Devolución de efectivo".to_string(),
            performed_by: Some("cash-report-test".to_string()),
        },
    )
    .await
    .unwrap();

    let sale_out: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM cash_movements \
         WHERE session_id = $1 AND movement_type = 'sale_out'",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let report = cash_service::get_session_report(&pool, session_id)
        .await
        .unwrap();
    assert_eq!(report.cash.cash_refunds, sale_out);
    assert_eq!(report.cash.cash_refunds, 10_000);
    assert_eq!(report.cash.expected_amount, 5_000);
}

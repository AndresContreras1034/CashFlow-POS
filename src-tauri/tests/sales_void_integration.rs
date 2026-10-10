use pos_lib::modules::{
    cash::{dto::CloseSessionDto, service as cash_service},
    sales::{
        dto::{CreateSaleDto, CreateSaleItemDto, CreateSalePaymentDto, SaleFilterDto, VoidSaleDto},
        models::{PaymentMethod, SaleStatus},
        service as sales_service,
    },
};
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(1);

async fn seed_variant(pool: &PgPool, price: i64, stock: i32) -> i32 {
    let suffix = SEQ.fetch_add(1, Ordering::Relaxed);
    let category_id: i32 = sqlx::query_scalar(
        "INSERT INTO categories (name) VALUES ($1) RETURNING id",
    )
    .bind(format!("void-category-{suffix}"))
    .fetch_one(pool)
    .await
    .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name, is_active) VALUES ($1, $2, TRUE) RETURNING id",
    )
    .bind(category_id)
    .bind(format!("void-product-{suffix}"))
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

async fn open_cash(pool: &PgPool) -> i32 {
    sqlx::query_scalar("INSERT INTO cash_sessions (opening_amount) VALUES (0) RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn close_cash(pool: &PgPool, session_id: i32, counted_amount: i64) {
    cash_service::close_session(
        pool,
        session_id,
        CloseSessionDto {
            counted_amount,
            closing_notes: None,
            closed_by: Some("void-test".to_string()),
        },
    )
    .await
    .unwrap();
}

fn sale_dto(
    variant_id: i32,
    payments: Vec<CreateSalePaymentDto>,
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
        discount: None,
        cash_received,
        notes: None,
        courtesy_reason: None,
        created_by: Some("void-test".to_string()),
        idempotency_key: None,
    }
}

fn payment(method: PaymentMethod, amount: i64) -> CreateSalePaymentDto {
    CreateSalePaymentDto { method, amount }
}

fn void_dto(target_status: SaleStatus, reason: &str) -> VoidSaleDto {
    VoidSaleDto {
        target_status,
        reason: reason.to_string(),
        performed_by: Some("void-operator".to_string()),
    }
}

async fn create_cash_sale(
    pool: &PgPool,
    variant_id: i32,
    cash_received: Option<i64>,
) -> i32 {
    sales_service::create_sale(
        pool,
        sale_dto(
            variant_id,
            vec![payment(PaymentMethod::Cash, 10_000)],
            cash_received,
        ),
    )
    .await
    .unwrap()
    .sale
    .id
}

async fn stock(pool: &PgPool, variant_id: i32) -> i32 {
    sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn v01_efectivo_con_vuelto_anula_y_cierra_en_cero(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_cash(&pool).await;
    let sale_id = create_cash_sale(&pool, variant_id, Some(20_000)).await;

    let voided = sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "Venta duplicada"),
    )
    .await
    .unwrap();

    assert_eq!(voided.sale.status, SaleStatus::Cancelled);
    assert_eq!(stock(&pool, variant_id).await, 5);
    let return_type: String = sqlx::query_scalar(
        "SELECT movement_type::TEXT FROM inventory_movements \
         WHERE sale_id = $1 AND movement_type = 'sale_return'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(return_type, "sale_return");
    let (amount, movement_type): (i64, String) = sqlx::query_as(
        "SELECT amount, movement_type::TEXT FROM cash_movements \
         WHERE sale_id = $1 AND movement_type = 'sale_out'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((amount, movement_type.as_str()), (10_000, "sale_out"));

    close_cash(&pool, session_id, 0).await;
    let expected: Option<i64> =
        sqlx::query_scalar("SELECT expected_amount FROM cash_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(expected, Some(0));
}

#[sqlx::test(migrations = "./migrations")]
async fn v02_pago_mixto_devuelve_solo_efectivo_neto(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let created = sales_service::create_sale(
        &pool,
        sale_dto(
            variant_id,
            vec![
                payment(PaymentMethod::Card, 4_000),
                payment(PaymentMethod::Cash, 6_000),
            ],
            Some(10_000),
        ),
    )
    .await
    .unwrap();
    sales_service::void_sale(
        &pool,
        created.sale.id,
        void_dto(SaleStatus::Refunded, "Devolución total"),
    )
    .await
    .unwrap();
    let amount: i64 = sqlx::query_scalar(
        "SELECT amount FROM cash_movements WHERE sale_id = $1 AND movement_type = 'sale_out'",
    )
    .bind(created.sale.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(amount, 6_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn v03_venta_de_tarjeta_se_devuelve_sin_turno_abierto(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_cash(&pool).await;
    let created = sales_service::create_sale(
        &pool,
        sale_dto(
            variant_id,
            vec![payment(PaymentMethod::Card, 10_000)],
            None,
        ),
    )
    .await
    .unwrap();
    close_cash(&pool, session_id, 0).await;
    let movements_before = count(&pool, "cash_movements").await;

    sales_service::void_sale(
        &pool,
        created.sale.id,
        void_dto(SaleStatus::Refunded, "Devolución tarjeta"),
    )
    .await
    .unwrap();

    assert_eq!(stock(&pool, variant_id).await, 5);
    assert_eq!(count(&pool, "cash_movements").await, movements_before);
}

#[sqlx::test(migrations = "./migrations")]
async fn v04_venta_en_efectivo_sin_turno_no_modifica_nada(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let session_id = open_cash(&pool).await;
    let sale_id = create_cash_sale(&pool, variant_id, None).await;
    close_cash(&pool, session_id, 10_000).await;
    let before_status: SaleStatus =
        sqlx::query_scalar("SELECT status FROM sales WHERE id = $1")
            .bind(sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let inventory_before = count(&pool, "inventory_movements").await;
    let cash_before = count(&pool, "cash_movements").await;
    let audit_before = count(&pool, "audit_events").await;

    let error = sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "No hay turno"),
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(error.contains("turno de caja"), "{error}");
    let after_status: SaleStatus =
        sqlx::query_scalar("SELECT status FROM sales WHERE id = $1")
            .bind(sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after_status, before_status);
    assert_eq!(stock(&pool, variant_id).await, 4);
    assert_eq!(count(&pool, "inventory_movements").await, inventory_before);
    assert_eq!(count(&pool, "cash_movements").await, cash_before);
    assert_eq!(count(&pool, "audit_events").await, audit_before);
}

#[sqlx::test(migrations = "./migrations")]
async fn v05_cortesia_repone_stock_sin_movimiento_de_caja(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let mut dto = sale_dto(variant_id, vec![], None);
    dto.discount = Some(10_000);
    dto.courtesy_reason = Some("Apoyo".to_string());
    let created = sales_service::create_sale(&pool, dto).await.unwrap();
    let cash_before = count(&pool, "cash_movements").await;

    sales_service::void_sale(
        &pool,
        created.sale.id,
        void_dto(SaleStatus::Cancelled, "Anulación de cortesía"),
    )
    .await
    .unwrap();

    assert_eq!(stock(&pool, variant_id).await, 5);
    assert_eq!(count(&pool, "cash_movements").await, cash_before);
}

#[sqlx::test(migrations = "./migrations")]
async fn v06_venta_de_turno_cerrado_se_devuelve_en_turno_actual(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let old_session = open_cash(&pool).await;
    let sale_id = create_cash_sale(&pool, variant_id, None).await;
    close_cash(&pool, old_session, 10_000).await;
    let new_session = open_cash(&pool).await;

    sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Refunded, "Devolución en nuevo turno"),
    )
    .await
    .unwrap();

    let refund_session: i32 = sqlx::query_scalar(
        "SELECT session_id FROM cash_movements WHERE sale_id = $1 AND movement_type = 'sale_out'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let old_expected: Option<i64> =
        sqlx::query_scalar("SELECT expected_amount FROM cash_sessions WHERE id = $1")
            .bind(old_session)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refund_session, new_session);
    assert_eq!(old_expected, Some(10_000));
}

#[sqlx::test(migrations = "./migrations")]
async fn v07_validaciones_no_escriben_fallos_de_auditoria(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale_id = create_cash_sale(&pool, variant_id, None).await;
    let invalid = [
        void_dto(SaleStatus::Cancelled, ""),
        void_dto(SaleStatus::Cancelled, "   "),
        void_dto(SaleStatus::Cancelled, &"x".repeat(201)),
        void_dto(SaleStatus::Completed, "No es un destino permitido"),
    ];
    for dto in invalid {
        assert!(sales_service::void_sale(&pool, sale_id, dto).await.is_err());
    }
    let failures: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE outcome::TEXT = 'failure'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failures, 0);
    let status: SaleStatus = sqlx::query_scalar("SELECT status FROM sales WHERE id = $1")
        .bind(sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, SaleStatus::Completed);
}

#[sqlx::test(migrations = "./migrations")]
async fn v08_segunda_anulacion_secuencial_se_rechaza(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale_id = sales_service::create_sale(
        &pool,
        sale_dto(variant_id, vec![payment(PaymentMethod::Card, 10_000)], None),
    )
    .await
    .unwrap()
    .sale
    .id;
    sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "Primera anulación"),
    )
    .await
    .unwrap();

    assert!(sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "Segunda anulación"),
    )
    .await
    .is_err());
    assert_eq!(stock(&pool, variant_id).await, 5);
    let return_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_movements WHERE sale_id = $1 AND movement_type = 'sale_return'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(return_count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn v09_anulaciones_concurrentes_reponen_una_sola_vez(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale_id = sales_service::create_sale(
        &pool,
        sale_dto(variant_id, vec![payment(PaymentMethod::Cash, 10_000)], None),
    )
    .await
    .unwrap()
    .sale
    .id;

    let (first, second) = tokio::join!(
        sales_service::void_sale(
            &pool,
            sale_id,
            void_dto(SaleStatus::Cancelled, "Concurrencia A")
        ),
        sales_service::void_sale(
            &pool,
            sale_id,
            void_dto(SaleStatus::Refunded, "Concurrencia B")
        )
    );
    assert_ne!(first.is_ok(), second.is_ok());
    assert_eq!(stock(&pool, variant_id).await, 5);
    let sale_out_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cash_movements WHERE sale_id = $1 AND movement_type = 'sale_out'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sale_out_count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn v10_fallo_de_auditoria_revierte_operacion_completa(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale_id = create_cash_sale(&pool, variant_id, None).await;
    let inventory_before = count(&pool, "inventory_movements").await;
    let cash_before = count(&pool, "cash_movements").await;
    sqlx::query(
        r#"CREATE FUNCTION fail_sale_void_audit() RETURNS trigger AS $$
           BEGIN RAISE EXCEPTION 'audit caido'; END;
           $$ LANGUAGE plpgsql"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER trg_fail_sale_void_audit BEFORE INSERT ON audit_events \
         FOR EACH ROW EXECUTE FUNCTION fail_sale_void_audit()",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "Debe revertirse"),
    )
    .await
    .is_err());
    let status: SaleStatus = sqlx::query_scalar("SELECT status FROM sales WHERE id = $1")
        .bind(sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, SaleStatus::Completed);
    assert_eq!(stock(&pool, variant_id).await, 4);
    assert_eq!(count(&pool, "inventory_movements").await, inventory_before);
    assert_eq!(count(&pool, "cash_movements").await, cash_before);
}

#[sqlx::test(migrations = "./migrations")]
async fn v11_detalle_y_listado_muestran_anulacion_sin_sumarla(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    open_cash(&pool).await;
    let sale_id = sales_service::create_sale(
        &pool,
        sale_dto(variant_id, vec![payment(PaymentMethod::Card, 10_000)], None),
    )
    .await
    .unwrap()
    .sale
    .id;
    let voided = sales_service::void_sale(
        &pool,
        sale_id,
        void_dto(SaleStatus::Cancelled, "Error de caja"),
    )
    .await
    .unwrap();
    assert_eq!(voided.sale.status, SaleStatus::Cancelled);
    assert_eq!(voided.sale.void_reason.as_deref(), Some("Error de caja"));
    assert_eq!(voided.sale.voided_by.as_deref(), Some("void-operator"));
    assert!(voided.sale.voided_at.is_some());

    let listed = sales_service::list_sales(
        &pool,
        SaleFilterDto {
            status: Some("cancelled".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].id, sale_id);
    assert_eq!(listed.total_amount, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn v12_venta_de_tarjeta_se_anula_con_o_sin_turno_sin_sale_out(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10_000, 5).await;
    let first_session = open_cash(&pool).await;
    let first_sale = sales_service::create_sale(
        &pool,
        sale_dto(
            variant_id,
            vec![payment(PaymentMethod::Card, 10_000)],
            None,
        ),
    )
    .await
    .unwrap();
    close_cash(&pool, first_session, 0).await;
    sales_service::void_sale(
        &pool,
        first_sale.sale.id,
        void_dto(SaleStatus::Cancelled, "Anulación sin turno abierto"),
    )
    .await
    .unwrap();
    assert_eq!(count(&pool, "cash_movements").await, 0);

    let second_session = open_cash(&pool).await;
    let second_sale = sales_service::create_sale(
        &pool,
        sale_dto(
            variant_id,
            vec![payment(PaymentMethod::Card, 10_000)],
            None,
        ),
    )
    .await
    .unwrap();
    let movements_before = count(&pool, "cash_movements").await;
    sales_service::void_sale(
        &pool,
        second_sale.sale.id,
        void_dto(SaleStatus::Refunded, "Devolución con turno abierto"),
    )
    .await
    .unwrap();

    assert_eq!(count(&pool, "cash_movements").await, movements_before);
    assert_eq!(stock(&pool, variant_id).await, 5);
    let open_status: String =
        sqlx::query_scalar("SELECT status FROM cash_sessions WHERE id = $1")
            .bind(second_session)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(open_status, "open");
}

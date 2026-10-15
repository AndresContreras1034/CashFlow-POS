//! Integration coverage for customers and customer assignment on sales.
//! Each sqlx test uses a temporary database and the repository migrations.

use pos_lib::modules::{
    customers::{
        dto::{CreateCustomerDto, CustomerFilterDto, SetCustomerActiveDto, UpdateCustomerDto},
        service as customers,
    },
    sales::{
        dto::{
            AssignSaleCustomerDto, CreateSaleDto, CreateSaleItemDto, CreateSalePaymentDto,
            SaleFilterDto,
        },
        models::PaymentMethod,
        service as sales,
    },
};
use sqlx::PgPool;

fn customer(name: &str) -> CreateCustomerDto {
    CreateCustomerDto {
        name: name.to_string(),
        document_type: None,
        document_number: None,
        phone: None,
        email: None,
        address: None,
        notes: None,
        created_by: Some("test".to_string()),
    }
}

fn customer_with_document(name: &str, kind: &str, number: &str) -> CreateCustomerDto {
    let mut dto = customer(name);
    dto.document_type = Some(kind.to_string());
    dto.document_number = Some(number.to_string());
    dto
}

async fn seed_variant(pool: &PgPool) -> i32 {
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ('customer-test') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name) VALUES ($1, 'customer-test-product') RETURNING id",
    )
    .bind(category_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price, stock)
         VALUES ($1, '{}'::jsonb, 10000, 10) RETURNING id",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn open_cash(pool: &PgPool) {
    sqlx::query("INSERT INTO cash_sessions (opening_amount) VALUES (0)")
        .execute(pool)
        .await
        .unwrap();
}

fn sale(variant_id: i32, customer_id: Option<i32>) -> CreateSaleDto {
    CreateSaleDto {
        customer_id,
        items: vec![CreateSaleItemDto {
            variant_id,
            quantity: 1,
            discount: None,
        }],
        payments: vec![CreateSalePaymentDto {
            method: PaymentMethod::Card,
            amount: 10_000,
        }],
        discount: None,
        cash_received: None,
        notes: None,
        courtesy_reason: None,
        created_by: Some("test".to_string()),
        idempotency_key: None,
    }
}

fn assign(customer_id: Option<i32>) -> AssignSaleCustomerDto {
    AssignSaleCustomerDto {
        customer_id,
        performed_by: Some("test".to_string()),
    }
}

async fn count(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn deactivate(pool: &PgPool, id: i32) {
    customers::set_customer_active(
        pool,
        id,
        SetCustomerActiveDto {
            is_active: false,
            updated_by: Some("test".to_string()),
        },
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn customer_normalization_and_duplicate_documents(pool: PgPool) {
    let mut dto = customer_with_document("  Ana Pérez  ", "cc", " 123 ");
    dto.phone = Some("   ".to_string());
    let created = customers::create_customer(&pool, dto).await.unwrap();
    assert_eq!(created.name, "Ana Pérez");
    assert_eq!(created.document_type.as_deref(), Some("CC"));
    assert_eq!(created.document_number.as_deref(), Some("123"));
    assert_eq!(created.phone, None);

    let duplicate =
        customers::create_customer(&pool, customer_with_document("Otra Ana", "CC", "123")).await;
    let duplicate_message = duplicate.unwrap_err().to_string();
    assert!(
        duplicate_message.contains("documento"),
        "{duplicate_message}"
    );
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM customers").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn customer_validation_edits_and_activation_are_audited(pool: PgPool) {
    assert!(customers::create_customer(&pool, customer("   "))
        .await
        .is_err());
    let mut incomplete_document = customer("Ana");
    incomplete_document.document_type = Some("CC".to_string());
    assert!(customers::create_customer(&pool, incomplete_document)
        .await
        .is_err());
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM customers").await, 0);

    let created = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let updated = customers::update_customer(
        &pool,
        created.id,
        UpdateCustomerDto {
            name: "Ana María".to_string(),
            document_type: None,
            document_number: None,
            phone: Some("3001234567".to_string()),
            email: None,
            address: None,
            notes: None,
            updated_by: Some("test".to_string()),
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.name, "Ana María");

    let changes: serde_json::Value =
        sqlx::query_scalar("SELECT changes FROM audit_events WHERE action = 'update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(changes["name"]["from"], "Ana");
    assert_eq!(changes["name"]["to"], "Ana María");
    assert!(changes.get("email").is_none());

    deactivate(&pool, created.id).await;
    deactivate(&pool, created.id).await;
    let reactivated = customers::set_customer_active(
        &pool,
        created.id,
        SetCustomerActiveDto {
            is_active: true,
            updated_by: Some("test".to_string()),
        },
    )
    .await
    .unwrap();
    assert!(reactivated.is_active);
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_events WHERE action = 'deactivate'"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_events WHERE action = 'activate'"
        )
        .await,
        1
    );

    let found = customers::list_customers(
        &pool,
        CustomerFilterDto {
            search: Some("maría".to_string()),
            is_active: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(found.total, 1);
    assert_eq!(found.data[0].customer.id, created.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn customer_name_lookup_deduplicates_ids_and_handles_empty_input(pool: PgPool) {
    let ana = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let beto = customers::create_customer(&pool, customer("Beto"))
        .await
        .unwrap();

    let names = customers::get_customer_names(&pool, vec![beto.id, ana.id, ana.id, 999_999])
        .await
        .unwrap();
    assert_eq!(names.len(), 2);
    assert!(customers::get_customer_names(&pool, vec![])
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn creating_sale_with_customer_updates_customer_statistics(pool: PgPool) {
    let customer = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let variant = seed_variant(&pool).await;
    open_cash(&pool).await;

    let created = sales::create_sale(&pool, sale(variant, Some(customer.id)))
        .await
        .unwrap();
    assert_eq!(created.sale.customer_id, Some(customer.id));

    let overview = customers::get_customer(&pool, customer.id).await.unwrap();
    assert_eq!(overview.sales_count, 1);
    assert_eq!(overview.total_spent, 10_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn assigning_changing_and_clearing_customer_only_changes_customer_id(pool: PgPool) {
    let ana = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let beto = customers::create_customer(&pool, customer("Beto"))
        .await
        .unwrap();
    let variant = seed_variant(&pool).await;
    open_cash(&pool).await;
    let created = sales::create_sale(&pool, sale(variant, None))
        .await
        .unwrap();

    let stock_before: i32 = sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant)
        .fetch_one(&pool)
        .await
        .unwrap();
    let assigned = sales::assign_customer(&pool, created.sale.id, assign(Some(ana.id)))
        .await
        .unwrap();
    assert_eq!(assigned.sale.customer_id, Some(ana.id));
    assert_eq!(assigned.sale.total, created.sale.total);

    sales::assign_customer(&pool, created.sale.id, assign(Some(beto.id)))
        .await
        .unwrap();
    let cleared = sales::assign_customer(&pool, created.sale.id, assign(None))
        .await
        .unwrap();
    assert_eq!(cleared.sale.customer_id, None);
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT stock FROM product_variants WHERE id = $1")
            .bind(variant)
            .fetch_one(&pool)
            .await
            .unwrap(),
        stock_before
    );
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM audit_events WHERE module = 'sales' AND action = 'assign_customer'")
            .await,
        3
    );

    let changes: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT changes FROM audit_events WHERE action = 'assign_customer' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(changes[0]["customer_id"]["from"].is_null());
    assert_eq!(changes[0]["customer_id"]["to"], ana.id);
    assert_eq!(changes[1]["customer_id"]["from"], ana.id);
    assert_eq!(changes[1]["customer_id"]["to"], beto.id);
    assert_eq!(changes[2]["customer_id"]["from"], beto.id);
    assert!(changes[2]["customer_id"]["to"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn assignment_rejects_missing_or_inactive_customer_and_is_idempotent(pool: PgPool) {
    let customer = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let variant = seed_variant(&pool).await;
    open_cash(&pool).await;
    let created = sales::create_sale(&pool, sale(variant, Some(customer.id)))
        .await
        .unwrap();

    sales::assign_customer(&pool, created.sale.id, assign(Some(customer.id)))
        .await
        .unwrap();
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_events WHERE action = 'assign_customer'"
        )
        .await,
        0
    );

    assert!(
        sales::assign_customer(&pool, created.sale.id, assign(Some(999_999)))
            .await
            .is_err()
    );
    deactivate(&pool, customer.id).await;
    assert!(
        sales::assign_customer(&pool, created.sale.id, assign(Some(customer.id)))
            .await
            .is_err()
    );
    assert!(sales::assign_customer(&pool, 999_999, assign(None))
        .await
        .is_err());

    let fetched = sales::get_sale(&pool, created.sale.id).await.unwrap();
    assert_eq!(fetched.sale.customer_id, Some(customer.id));
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_events WHERE action = 'assign_customer'"
        )
        .await,
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sales_filter_can_select_customer(pool: PgPool) {
    let ana = customers::create_customer(&pool, customer("Ana"))
        .await
        .unwrap();
    let beto = customers::create_customer(&pool, customer("Beto"))
        .await
        .unwrap();
    let variant = seed_variant(&pool).await;
    open_cash(&pool).await;
    sales::create_sale(&pool, sale(variant, Some(ana.id)))
        .await
        .unwrap();
    sales::create_sale(&pool, sale(variant, Some(beto.id)))
        .await
        .unwrap();
    sales::create_sale(&pool, sale(variant, None))
        .await
        .unwrap();

    let result = sales::list_sales(
        &pool,
        SaleFilterDto {
            customer_id: Some(ana.id),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(result.total, 1);
    assert!(result
        .data
        .iter()
        .all(|item| item.customer_id == Some(ana.id)));
}

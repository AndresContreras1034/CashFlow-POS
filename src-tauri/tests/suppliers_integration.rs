use pos_lib::modules::suppliers::{
    purchase_lists::{self, AddItemsDto, PurchaseListDetail, UpdateItemDto},
    receive::{self, ReceiveLineDto, ReceiveListDto},
};
use sqlx::PgPool;

async fn seed_variant(pool: &PgPool) -> i32 {
    let category_id: i32 = sqlx::query_scalar(
        "INSERT INTO categories (name) VALUES ('purchase-list-test') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name) VALUES ($1, 'Aretes') RETURNING id",
    )
    .bind(category_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price, cost, stock, stock_min)
         VALUES ($1, '{}'::jsonb, 20000, 10000, 0, 3) RETURNING id",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn new_supplier(pool: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO suppliers (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn variant(pool: &PgPool, name: &str, stock: i32, stock_min: i32, cost: i64) -> i32 {
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
            .bind(format!("purchase-receive-{name}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 =
        sqlx::query_scalar("INSERT INTO products (category_id, name) VALUES ($1, $2) RETURNING id")
            .bind(category_id)
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants
             (product_id, attributes, price, cost, stock, stock_min)
         VALUES ($1, '{}'::jsonb, 200_000, $2, $3, $4)
         RETURNING id",
    )
    .bind(product_id)
    .bind(cost)
    .bind(stock)
    .bind(stock_min)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn link(pool: &PgPool, supplier_id: i32, variant_id: i32) {
    sqlx::query(
        "INSERT INTO supplier_variants (supplier_id, variant_id, is_preferred)
         VALUES ($1, $2, TRUE)",
    )
    .bind(supplier_id)
    .bind(variant_id)
    .execute(pool)
    .await
    .unwrap();
}

fn find(detail: &PurchaseListDetail, variant_id: i32) -> &purchase_lists::PurchaseItem {
    detail
        .items
        .iter()
        .find(|item| item.variant_id == variant_id)
        .unwrap()
}

async fn audit_count(pool: &PgPool, action: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events
         WHERE module::text = 'suppliers' AND action = $1",
    )
    .bind(action)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn create_list(pool: &PgPool, title: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO purchase_lists (title) VALUES ($1) RETURNING id")
        .bind(title)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn edit(quantity: i32, supplier_id: Option<i32>) -> UpdateItemDto {
    UpdateItemDto {
        quantity,
        supplier_id,
        note: None,
        updated_by: Some("test".to_string()),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_line_keeps_its_deactivated_supplier_when_only_the_quantity_changes(pool: PgPool) {
    let variant_id = seed_variant(&pool).await;
    let supplier_id = new_supplier(&pool, "Norte").await;
    let other_inactive_supplier = new_supplier(&pool, "Cerrado").await;
    let list_id = create_list(&pool, "Lista de prueba").await;
    let item_id: i32 = sqlx::query_scalar(
        "INSERT INTO purchase_list_items (list_id, variant_id, supplier_id, quantity)
         VALUES ($1, $2, $3, 3) RETURNING id",
    )
    .bind(list_id)
    .bind(variant_id)
    .bind(supplier_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query("UPDATE suppliers SET is_active = FALSE WHERE id IN ($1, $2)")
        .bind(supplier_id)
        .bind(other_inactive_supplier)
        .execute(&pool)
        .await
        .unwrap();

    let updated = purchase_lists::update_purchase_item(&pool, item_id, edit(7, Some(supplier_id)))
        .await
        .unwrap();
    assert_eq!(updated.quantity, 7);
    assert_eq!(
        updated.supplier.as_ref().map(|supplier| supplier.id),
        Some(supplier_id)
    );

    let change_to_inactive = purchase_lists::update_purchase_item(
        &pool,
        item_id,
        edit(7, Some(other_inactive_supplier)),
    )
    .await;
    assert!(change_to_inactive.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn receiving_a_list_adds_stock_averages_cost_and_cannot_repeat(pool: PgPool) {
    let aretes = variant(&pool, "Aretes", 2, 3, 100_000).await;
    let collar = variant(&pool, "Collar", 5, 1, 50_000).await;
    let norte = new_supplier(&pool, "Norte").await;
    link(&pool, norte, aretes).await;

    let list_id = create_list(&pool, "Ida a Bogotá").await;
    purchase_lists::add_purchase_items(
        &pool,
        list_id,
        AddItemsDto {
            variant_ids: vec![aretes, collar],
            updated_by: None,
        },
    )
    .await
    .unwrap();
    let detail = purchase_lists::get_purchase_list(&pool, list_id)
        .await
        .unwrap();
    let aretes_item = find(&detail, aretes).item_id;
    let collar_item = find(&detail, collar).item_id;

    let receive_dto = |lines: Vec<ReceiveLineDto>| ReceiveListDto {
        lines,
        updated_by: None,
    };

    assert!(receive::receive_purchase_list(
        &pool,
        list_id,
        receive_dto(vec![
            ReceiveLineDto {
                item_id: aretes_item,
                quantity: 0,
                unit_cost: None
            },
            ReceiveLineDto {
                item_id: collar_item,
                quantity: 0,
                unit_cost: None
            },
        ]),
    )
    .await
    .is_err());

    let result = receive::receive_purchase_list(
        &pool,
        list_id,
        receive_dto(vec![
            ReceiveLineDto {
                item_id: aretes_item,
                quantity: 8,
                unit_cost: Some(150_000),
            },
            ReceiveLineDto {
                item_id: collar_item,
                quantity: 0,
                unit_cost: None,
            },
        ]),
    )
    .await
    .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM purchase_lists WHERE id = $1")
        .bind(list_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "received");
    assert_eq!((result.received_lines, result.skipped_lines), (1, 1));
    assert_eq!(result.units, 8);
    assert_eq!(result.total_cost, 1_200_000);
    assert_eq!(result.cost_changes.len(), 1);
    assert_eq!(
        (result.cost_changes[0].from, result.cost_changes[0].to),
        (100_000, 140_000)
    );

    let (stock, cost): (i32, i64) =
        sqlx::query_as("SELECT stock, cost FROM product_variants WHERE id = $1")
            .bind(aretes)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((stock, cost), (10, 140_000));

    let movement_count = |variant_id: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM inventory_movements
                 WHERE variant_id = $1 AND movement_type = 'purchase'",
            )
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(movement_count(aretes).await, 1);
    assert_eq!(movement_count(collar).await, 0);

    let (collar_stock, collar_cost): (i32, i64) =
        sqlx::query_as("SELECT stock, cost FROM product_variants WHERE id = $1")
            .bind(collar)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((collar_stock, collar_cost), (5, 50_000));

    let (last_cost, bought): (Option<i64>, bool) = sqlx::query_as(
        "SELECT last_cost, last_purchase_at IS NOT NULL FROM supplier_variants
         WHERE supplier_id = $1 AND variant_id = $2",
    )
    .bind(norte)
    .bind(aretes)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(last_cost, Some(150_000));
    assert!(bought);

    assert_eq!(audit_count(&pool, "purchase_list_receive").await, 1);

    assert!(receive::receive_purchase_list(
        &pool,
        list_id,
        receive_dto(vec![
            ReceiveLineDto {
                item_id: aretes_item,
                quantity: 8,
                unit_cost: Some(150_000)
            },
            ReceiveLineDto {
                item_id: collar_item,
                quantity: 1,
                unit_cost: None
            },
        ]),
    )
    .await
    .is_err());
    let stock_after: i32 = sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(aretes)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stock_after, 10);
    assert_eq!(audit_count(&pool, "purchase_list_receive").await, 1);
}

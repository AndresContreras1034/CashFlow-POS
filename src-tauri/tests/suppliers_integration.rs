use pos_lib::modules::suppliers::purchase_lists::{self, CreatePurchaseListDto, UpdateItemDto};
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
    let list = purchase_lists::create_purchase_list(
        &pool,
        CreatePurchaseListDto {
            title: "Lista de prueba".to_string(),
            notes: None,
            created_by: Some("test".to_string()),
        },
    )
    .await
    .unwrap();
    let item_id: i32 = sqlx::query_scalar(
        "INSERT INTO purchase_list_items (list_id, variant_id, supplier_id, quantity)
         VALUES ($1, $2, $3, 3) RETURNING id",
    )
    .bind(list.id)
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

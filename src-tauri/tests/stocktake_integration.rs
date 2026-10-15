use pos_lib::modules::{
    inventory::{dto::StockOutDto, models::MovementType, repository as inventory_repository},
    stocktake::{dto::CreateStocktakeDto, service as stocktake_service},
};
use sqlx::PgPool;

async fn seed_variant(pool: &PgPool, stock: i32) -> i32 {
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ('stocktake test') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
    let product_id: i32 = sqlx::query_scalar(
        "INSERT INTO products (category_id, name) VALUES ($1, 'product') RETURNING id",
    )
    .bind(category_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO product_variants (product_id, attributes, price, stock)
         VALUES ($1, '{}'::jsonb, 10000, $2) RETURNING id",
    )
    .bind(product_id)
    .bind(stock)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn start(pool: &PgPool) -> i32 {
    stocktake_service::start_stocktake(
        pool,
        CreateStocktakeDto {
            category_id: None,
            notes: None,
            created_by: None,
        },
    )
    .await
    .unwrap()
    .id
}

#[sqlx::test(migrations = "./migrations")]
async fn applying_count_keeps_a_sale_made_after_the_count(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10).await;
    let stocktake_id = start(&pool).await;

    stocktake_service::set_count(&pool, stocktake_id, variant_id, Some(8))
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    inventory_repository::apply_stock_out(
        &mut tx,
        StockOutDto {
            variant_id,
            quantity: 3,
            unit_cost: None,
            reason: None,
            notes: None,
            created_by: Some("cashier".into()),
            sale_id: None,
        },
        MovementType::Sale,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    stocktake_service::apply_stocktake(&pool, stocktake_id, None)
        .await
        .unwrap();

    let stock: i32 = sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stock, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_scan_increments_are_atomic(pool: PgPool) {
    let variant_id = seed_variant(&pool, 10).await;
    let stocktake_id = start(&pool).await;

    let (first, second) = tokio::join!(
        stocktake_service::increment_count(&pool, stocktake_id, variant_id),
        stocktake_service::increment_count(&pool, stocktake_id, variant_id),
    );
    assert_eq!(first.unwrap(), 1);
    assert_eq!(second.unwrap(), 2);
}

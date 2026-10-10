use pos_lib::modules::inventory::{
    dto::{CreateVariantDto, UpdateVariantDto},
    service,
};
use serde_json::json;
use sqlx::PgPool;

async fn seed_product(pool: &PgPool) -> i32 {
    let category_id: i32 =
        sqlx::query_scalar("INSERT INTO categories (name) VALUES ('c') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query_scalar("INSERT INTO products (category_id, name) VALUES ($1, 'p') RETURNING id")
        .bind(category_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn create_dto(product_id: i32, price: i64, cost: Option<i64>) -> CreateVariantDto {
    CreateVariantDto {
        product_id,
        attributes: json!({}),
        sku: None,
        barcode: None,
        price,
        cost,
        stock: None,
        stock_min: None,
        allow_negative: None,
    }
}

fn update_dto(price: Option<i64>, cost: Option<i64>) -> UpdateVariantDto {
    UpdateVariantDto {
        attributes: None,
        sku: None,
        barcode: None,
        price,
        cost,
        stock_min: None,
        allow_negative: None,
        is_active: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn n01_precio_entero_en_cop_se_acepta(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    service::create_variant(&pool, create_dto(product_id, 4_500_000, Some(2_000_000)))
        .await
        .expect("precio válido");
}

#[sqlx::test(migrations = "./migrations")]
async fn n02_precio_con_fraccion_se_rechaza_al_crear(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let message = service::create_variant(&pool, create_dto(product_id, 4_500_050, None))
        .await
        .expect_err("fracción en COP")
        .to_string();
    assert!(message.contains("fracciones"), "{message}");
}

#[sqlx::test(migrations = "./migrations")]
async fn n03_costo_con_fraccion_se_rechaza_al_crear(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let message = service::create_variant(&pool, create_dto(product_id, 4_500_000, Some(150)))
        .await
        .expect_err("costo con fracción")
        .to_string();
    assert!(message.contains("fracciones"), "{message}");
}

#[sqlx::test(migrations = "./migrations")]
async fn n04_precio_con_fraccion_se_rechaza_al_editar(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 1_000_000, None))
        .await
        .unwrap();
    let message = service::update_variant(&pool, variant.id, update_dto(Some(1_000_001), None))
        .await
        .expect_err("fracción al editar")
        .to_string();
    assert!(message.contains("fracciones"), "{message}");
}

#[sqlx::test(migrations = "./migrations")]
async fn n05_con_dos_decimales_se_aceptan_centavos(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET currency = 'USD', currency_decimals = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let product_id = seed_product(&pool).await;
    service::create_variant(&pool, create_dto(product_id, 1_999, Some(1_050)))
        .await
        .expect("centavos válidos en USD");
}

#[sqlx::test(migrations = "./migrations")]
async fn n06_editar_otros_campos_no_exige_precio(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 1_000_000, None))
        .await
        .unwrap();
    service::update_variant(&pool, variant.id, update_dto(None, None))
        .await
        .expect("sin precio ni costo no valida importes");
}

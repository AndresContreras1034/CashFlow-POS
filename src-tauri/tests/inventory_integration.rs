use pos_lib::modules::inventory::{
    dto::{CreateVariantDto, KardexFilterDto, NullableField, StockEntryDto, UpdateVariantDto},
    repository, service,
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
        sku: NullableField::Absent,
        barcode: NullableField::Absent,
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

#[sqlx::test(migrations = "./migrations")]
async fn optional_product_fields_can_be_cleared_with_null_or_empty_strings(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    sqlx::query(
        "UPDATE products SET description = 'description', brand = 'brand', image_url = 'image' WHERE id = $1",
    )
    .bind(product_id)
    .execute(&pool)
    .await
    .unwrap();

    let dto = serde_json::from_value(json!({
        "description": null,
        "brand": "",
        "image_url": ""
    }))
    .unwrap();
    service::update_product(&pool, product_id, dto)
        .await
        .unwrap();

    let fields: (Option<String>, Option<String>, Option<String>) =
        sqlx::query_as("SELECT description, brand, image_url FROM products WHERE id = $1")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(fields, (None, None, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn optional_variant_codes_can_be_cleared(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(
        &pool,
        CreateVariantDto {
            product_id,
            attributes: json!({"color": "blue"}),
            sku: Some("SKU-1".into()),
            barcode: Some("BARCODE-1".into()),
            price: 1_000_000,
            cost: None,
            stock: None,
            stock_min: None,
            allow_negative: None,
        },
    )
    .await
    .unwrap();

    let dto = serde_json::from_value(json!({"sku": "", "barcode": null})).unwrap();
    let updated = service::update_variant(&pool, variant.id, dto)
        .await
        .unwrap();
    assert_eq!(updated.sku, None);
    let barcode: Option<String> =
        sqlx::query_scalar("SELECT barcode FROM product_variants WHERE id = $1")
            .bind(variant.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(barcode, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn stock_entry_rejects_excessive_quantity_and_invalid_cost(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 1_000_000, None))
        .await
        .unwrap();

    for (quantity, unit_cost) in [(1_000_001, None), (1, Some(-100)), (1, Some(150))] {
        let result = service::register_stock_entry(
            &pool,
            StockEntryDto {
                variant_id: variant.id,
                quantity,
                unit_cost,
                notes: None,
                created_by: None,
            },
        )
        .await;
        assert!(result.is_err());
    }

    let stock: i32 = sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stock, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn stock_entry_accepts_cent_precision_when_currency_has_two_decimals(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET currency_decimals = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 10_000, None))
        .await
        .unwrap();

    service::register_stock_entry(
        &pool,
        StockEntryDto {
            variant_id: variant.id,
            quantity: 1,
            unit_cost: Some(105),
            notes: None,
            created_by: None,
        },
    )
    .await
    .expect("cent-based cost is valid with two currency decimals");
}

#[sqlx::test(migrations = "./migrations")]
async fn stock_entry_overflow_is_rejected_without_changing_stock(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 1_000_000, None))
        .await
        .unwrap();
    sqlx::query("UPDATE product_variants SET stock = $1 WHERE id = $2")
        .bind(i32::MAX)
        .bind(variant.id)
        .execute(&pool)
        .await
        .unwrap();

    let result = service::register_stock_entry(
        &pool,
        StockEntryDto {
            variant_id: variant.id,
            quantity: 1,
            unit_cost: None,
            notes: None,
            created_by: None,
        },
    )
    .await;
    assert!(result.is_err());

    let stock: i32 = sqlx::query_scalar("SELECT stock FROM product_variants WHERE id = $1")
        .bind(variant.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stock, i32::MAX);
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_variant_code_returns_a_friendly_conflict(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let first = service::create_variant(
        &pool,
        CreateVariantDto {
            product_id,
            attributes: json!({"color": "red"}),
            sku: Some("UNIQUE-SKU".into()),
            barcode: None,
            price: 1_000_000,
            cost: None,
            stock: None,
            stock_min: None,
            allow_negative: None,
        },
    )
    .await
    .unwrap();
    let second = service::create_variant(
        &pool,
        CreateVariantDto {
            product_id,
            attributes: json!({"color": "blue"}),
            sku: Some("SECOND-SKU".into()),
            barcode: None,
            price: 1_000_000,
            cost: None,
            stock: None,
            stock_min: None,
            allow_negative: None,
        },
    )
    .await
    .unwrap();
    let duplicate = service::update_variant(
        &pool,
        second.id,
        serde_json::from_value(json!({"sku": "UNIQUE-SKU"})).unwrap(),
    )
    .await
    .unwrap_err();

    assert_eq!(
        duplicate.to_string(),
        "Ya existe un registro con esos datos"
    );
    let sku: String = sqlx::query_scalar("SELECT sku FROM product_variants WHERE id = $1")
        .bind(first.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sku, "UNIQUE-SKU");
}

#[sqlx::test(migrations = "./migrations")]
async fn kardex_orders_equal_timestamps_by_id_and_paginates(pool: PgPool) {
    let product_id = seed_product(&pool).await;
    let variant = service::create_variant(&pool, create_dto(product_id, 1_000_000, None))
        .await
        .unwrap();
    let timestamp = chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    for quantity in 1..=3 {
        sqlx::query(
            "INSERT INTO inventory_movements
                 (variant_id, movement_type, quantity, stock_before, stock_after, unit_cost, created_by, created_at)
             VALUES ($1, 'manual_in', $2, 0, $2, 0, 'test', $3)",
        )
        .bind(variant.id)
        .bind(quantity)
        .bind(timestamp)
        .execute(&pool)
        .await
        .unwrap();
    }

    let filter = KardexFilterDto {
        variant_id: Some(variant.id),
        product_id: None,
        movement_type: None,
        date_from: None,
        date_to: None,
        page: Some(1),
        page_size: Some(2),
    };
    let (rows, total) = repository::get_kardex(&pool, &filter).await.unwrap();

    assert_eq!(total, 3);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].id > rows[1].id);
}

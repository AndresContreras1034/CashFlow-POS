#[tokio::test]
async fn test_list_categories() {
    dotenvy::dotenv().ok();
    let pool = pos_lib::db::init_db().await.unwrap();
    let result = pos_lib::modules::inventory::service::list_categories(&pool).await;
    assert!(result.is_ok());
    println!("{:?}", result.unwrap());
}

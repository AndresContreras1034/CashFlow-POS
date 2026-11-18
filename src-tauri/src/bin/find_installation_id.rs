#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_path(".env")?;
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&database_url).await?;
    let installation_id: String =
        sqlx::query_scalar("SELECT installation_id FROM license_state WHERE id = 1")
            .fetch_one(&pool)
            .await?;
    println!("{installation_id}");
    pool.close().await;
    Ok(())
}

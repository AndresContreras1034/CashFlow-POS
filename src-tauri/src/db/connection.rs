use sqlx::{postgres::PgPoolOptions, Connection, PgConnection, PgPool};
use std::time::Duration;

/// Tipo central del pool — se comparte via tauri State<DbPool>
pub type DbPool = PgPool;

/// Inicializa el pool de conexiones y corre las migraciones automáticamente.
/// Se llama una sola vez al arrancar la app en `lib.rs`.
pub async fn init_db() -> Result<DbPool, sqlx::Error> {
    // Leer DATABASE_URL desde .env o variable de entorno del sistema
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        sqlx::Error::Configuration(
            "DATABASE_URL no está definida. Crea un archivo .env en src-tauri/".into(),
        )
    })?;

    tracing::info!("Conectando a PostgreSQL...");

    // Sonda previa: expone el error real antes de crear el pool.
    PgConnection::connect(&database_url).await?.close().await?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .connect(&database_url)
        .await?;

    tracing::info!(ok = true, "Pool de conexiones creado");

    tracing::info!("Ejecutando migraciones...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!(ok = true, "Migraciones aplicadas");

    Ok(pool)
}

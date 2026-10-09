use sqlx::{postgres::PgPoolOptions, Connection, PgConnection, PgPool};
use std::time::{Duration, Instant};

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

/// Foto de salud tomada una vez al arrancar (solo para el panel de desarrollo).
#[derive(Debug, Clone)]
pub struct DbHealth {
    pub database: String,
    /// Texto completo de `version()`; el panel lo acorta.
    pub server_version: String,
    pub latency: Duration,
}

/// Consulta ligera con latencia medida. No toca datos de negocio.
pub async fn probe_health(pool: &DbPool) -> Result<DbHealth, sqlx::Error> {
    let started = Instant::now();
    let (database, server_version): (String, String) =
        sqlx::query_as("SELECT current_database(), version()")
            .fetch_one(pool)
            .await?;
    Ok(DbHealth {
        database,
        server_version,
        latency: started.elapsed(),
    })
}

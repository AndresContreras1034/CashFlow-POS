use std::sync::Arc;

use serde::Serialize;
use sqlx::PgPool;
use tauri::State;

use super::models::{DeveloperEvent, DeveloperStatus};
use super::state::DeveloperModeState;
use crate::db::connection::probe_health;
use crate::logging::{clean_value, effective_filter, short_pg_version};

#[tauri::command]
pub async fn set_developer_mode(
    state: State<'_, Arc<DeveloperModeState>>,
    enabled: bool,
) -> Result<(), String> {
    state.set_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub async fn get_developer_status(
    state: State<'_, Arc<DeveloperModeState>>,
) -> Result<DeveloperStatus, String> {
    Ok(state.status())
}

#[tauri::command]
pub async fn get_developer_events(
    state: State<'_, Arc<DeveloperModeState>>,
) -> Result<Vec<DeveloperEvent>, String> {
    Ok(state.snapshot())
}

#[tauri::command]
pub async fn clear_developer_events(
    state: State<'_, Arc<DeveloperModeState>>,
) -> Result<(), String> {
    state.clear();
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct HealthSnapshot {
    pub database: String,
    pub server_version: String,
    pub latency_ms: f64,
    pub db_time: String,
    pub db_timezone: String,
    pub pool_size: u32,
    pub pool_idle: usize,
    pub pool_max: u32,
    pub app_version: String,
    pub profile: String,
    pub log_filter: String,
}

/// Solo lectura: dos consultas ligeras y contadores del pool.
#[tauri::command]
pub async fn get_health(pool: State<'_, PgPool>) -> Result<HealthSnapshot, String> {
    let pool: &PgPool = pool.inner();

    let health = probe_health(pool)
        .await
        .map_err(|error| clean_value(&error.to_string()))?;

    let (db_time, db_timezone): (chrono::DateTime<chrono::Utc>, String) =
        sqlx::query_as("SELECT now(), current_setting('TimeZone')")
            .fetch_one(pool)
            .await
            .map_err(|error| clean_value(&error.to_string()))?;

    Ok(HealthSnapshot {
        database: health.database,
        server_version: short_pg_version(&health.server_version),
        latency_ms: health.latency.as_secs_f64() * 1000.0,
        db_time: db_time.to_rfc3339(),
        db_timezone,
        pool_size: pool.size(),
        pool_idle: pool.num_idle(),
        pool_max: pool.options().get_max_connections(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string(),
        log_filter: clean_value(&effective_filter(std::env::var("RUST_LOG").ok().as_deref())),
    })
}

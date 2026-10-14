//! Tarea de fondo: ejecuta el backup a la hora programada y recupera los
//! que se perdieron porque el PC estaba apagado (catch-up).

use std::time::Duration;

use chrono::{Local, Utc};
use sqlx::PgPool;
use tauri::AppHandle;

use crate::errors::app_error::AppError;
use crate::modules::backup::{
    handlers::default_backup_dir,
    models::BackupKind,
    notify, repository, service,
    status::{self, AlertLevel},
};

const TICK: Duration = Duration::from_secs(60);
const STARTUP_DELAY: Duration = Duration::from_secs(120);
/// Tras un fallo no se reintenta antes de este tiempo.
const RETRY_AFTER_MINUTES: i64 = 30;
/// Hasta cuántos minutos de retraso cuenta como "a su hora" y no como catch-up.
const ON_TIME_MINUTES: i64 = 10;

pub fn spawn(app: AppHandle, pool: PgPool) {
    tauri::async_runtime::spawn(async move {
        run(app, pool).await;
    });
}

async fn run(app: AppHandle, pool: PgPool) {
    // Al arrancar se espera un poco para no competir con la carga inicial,
    // salvo que ya estemos en nivel crítico.
    let critical = matches!(
        status::get(&pool).await,
        Ok(s) if s.level == AlertLevel::Critical
    );
    if !critical {
        tokio::time::sleep(STARTUP_DELAY).await;
    }

    loop {
        if let Err(e) = tick(&app, &pool).await {
            tracing::warn!(error = %e, "Scheduler de backup: error en la comprobación");
        }
        tokio::time::sleep(TICK).await;
    }
}

async fn tick(app: &AppHandle, pool: &PgPool) -> Result<(), AppError> {
    notify::retry_pending(pool).await;

    let settings = service::get_settings(pool).await?;
    if !settings.auto_enabled || service::is_running() {
        return Ok(());
    }

    let now_local = Local::now().naive_local();
    let Some(due_naive) = status::last_due(now_local, settings.schedule_time, settings.schedule_days)
    else {
        return Ok(());
    };
    let Some(due) = status::local_to_utc(due_naive) else {
        return Ok(());
    };

    let summary = repository::history_summary(pool)
        .await
        .map_err(AppError::from)?;

    let covered = summary.last_success.is_some_and(|at| at >= due);
    let hours = summary
        .last_success
        .map(|at| (Utc::now() - at).num_hours().max(0));
    let critical = status::compute_level(hours, settings.warn_after_days, settings.critical_after_days)
        == AlertLevel::Critical;

    if covered && !critical {
        return Ok(());
    }

    // Reintento escalonado después de un fallo.
    if summary.last_attempt_status.as_deref() == Some("failed") {
        if let Some(at) = summary.last_attempt_at {
            if Utc::now() - at < chrono::Duration::minutes(RETRY_AFTER_MINUTES) {
                return Ok(());
            }
        }
    }

    let lateness = Utc::now() - due;
    let kind = if !covered && lateness.num_minutes() <= ON_TIME_MINUTES {
        BackupKind::Scheduled
    } else {
        BackupKind::CatchUp
    };

    let dir = default_backup_dir(app).map_err(AppError::Internal)?;
    tracing::info!(kind = kind.as_str(), critical, "Iniciando backup automático");

    match service::run_backup(pool, &dir, kind, None).await {
        Ok(_) => {}
        // Si coincidió con uno manual, simplemente se omite.
        Err(e) => tracing::warn!(error = %e, "Backup automático no completado"),
    }
    Ok(())
}

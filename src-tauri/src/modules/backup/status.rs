//! Días sin backup, nivel de alerta y cálculo de horas programadas.
//! Las horas programadas se interpretan en la hora local del PC.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::backup::{repository, service};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Ok,
    Notice,
    Warning,
    Critical,
}

#[derive(Debug, Serialize)]
pub struct BackupStatus {
    pub level: AlertLevel,
    pub auto_enabled: bool,
    pub running: bool,
    pub last_success_at: Option<DateTime<Utc>>,
    pub hours_since_success: Option<i64>,
    pub days_since_success: Option<i64>,
    pub last_attempt_failed: bool,
    pub last_error: Option<String>,
    pub failures_since_success: i64,
    pub next_run_at: Option<DateTime<Utc>>,
    pub warn_after_days: i32,
    pub critical_after_days: i32,
}

/// Sin ningún backup exitoso => advertencia.
pub fn compute_level(hours: Option<i64>, warn: i32, critical: i32) -> AlertLevel {
    let Some(hours) = hours else {
        return AlertLevel::Warning;
    };
    let days = hours.max(0) / 24;
    if days >= critical as i64 {
        AlertLevel::Critical
    } else if days >= warn as i64 {
        AlertLevel::Warning
    } else if days >= 1 {
        AlertLevel::Notice
    } else {
        AlertLevel::Ok
    }
}

fn allowed(mask: i16, date: NaiveDate) -> bool {
    mask & (1i16 << date.weekday().num_days_from_monday()) != 0
}

/// Última hora programada que ya pasó (hasta 7 días atrás).
pub fn last_due(now: NaiveDateTime, time: NaiveTime, mask: i16) -> Option<NaiveDateTime> {
    (0..=7).find_map(|back| {
        let date = now.date() - Duration::days(back);
        let candidate = date.and_time(time);
        (allowed(mask, date) && candidate <= now).then_some(candidate)
    })
}

/// Próxima hora programada (hasta 7 días adelante).
pub fn next_due(now: NaiveDateTime, time: NaiveTime, mask: i16) -> Option<NaiveDateTime> {
    (0..=7).find_map(|ahead| {
        let date = now.date() + Duration::days(ahead);
        let candidate = date.and_time(time);
        (allowed(mask, date) && candidate > now).then_some(candidate)
    })
}

pub fn local_to_utc(naive: NaiveDateTime) -> Option<DateTime<Utc>> {
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
}

pub async fn get(pool: &PgPool) -> Result<BackupStatus, AppError> {
    let settings = service::get_settings(pool).await?;
    let summary = repository::history_summary(pool)
        .await
        .map_err(AppError::from)?;

    let hours = summary
        .last_success
        .map(|at| (Utc::now() - at).num_hours().max(0));
    let last_attempt_failed = summary.last_attempt_status.as_deref() == Some("failed");

    let next_run_at = if settings.auto_enabled {
        next_due(
            Local::now().naive_local(),
            settings.schedule_time,
            settings.schedule_days,
        )
        .and_then(local_to_utc)
    } else {
        None
    };

    Ok(BackupStatus {
        level: compute_level(hours, settings.warn_after_days, settings.critical_after_days),
        auto_enabled: settings.auto_enabled,
        running: service::is_running(),
        last_success_at: summary.last_success,
        hours_since_success: hours,
        days_since_success: hours.map(|h| h / 24),
        last_attempt_failed,
        last_error: if last_attempt_failed { summary.last_error } else { None },
        failures_since_success: summary.failures_since_success,
        next_run_at,
        warn_after_days: settings.warn_after_days,
        critical_after_days: settings.critical_after_days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: i16 = 127;
    const MONDAY: i16 = 1;

    fn dt(day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
    }

    fn nine_pm() -> NaiveTime {
        NaiveTime::from_hms_opt(21, 0, 0).unwrap()
    }

    #[test]
    fn levels_follow_days_without_backup() {
        assert_eq!(compute_level(Some(13), 2, 4), AlertLevel::Ok);
        assert_eq!(compute_level(Some(30), 2, 4), AlertLevel::Notice);
        assert_eq!(compute_level(Some(48), 2, 4), AlertLevel::Warning);
        assert_eq!(compute_level(Some(95), 2, 4), AlertLevel::Warning);
        assert_eq!(compute_level(Some(96), 2, 4), AlertLevel::Critical);
        assert_eq!(compute_level(None, 2, 4), AlertLevel::Warning);
    }

    // 13/10/2026 es martes; 12/10 es lunes.
    #[test]
    fn last_due_before_and_after_the_time() {
        assert_eq!(last_due(dt(13, 10), nine_pm(), ALL), Some(dt(12, 21)));
        assert_eq!(last_due(dt(13, 22), nine_pm(), ALL), Some(dt(13, 21)));
    }

    #[test]
    fn last_due_respects_weekdays() {
        assert_eq!(last_due(dt(13, 10), nine_pm(), MONDAY), Some(dt(12, 21)));
        assert_eq!(last_due(dt(13, 10), nine_pm(), 0), None);
    }

    #[test]
    fn next_due_respects_weekdays() {
        assert_eq!(next_due(dt(13, 10), nine_pm(), ALL), Some(dt(13, 21)));
        assert_eq!(next_due(dt(13, 22), nine_pm(), ALL), Some(dt(14, 21)));
        assert_eq!(next_due(dt(13, 10), nine_pm(), MONDAY), Some(dt(19, 21)));
    }
}

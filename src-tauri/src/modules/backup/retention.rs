//! Rotación: por cada bucket (día, semana ISO, mes) se conserva el backup más
//! reciente, para los N buckets más recientes que tengan backups.
//! El resultado es la unión de las tres políticas; el más nuevo nunca se borra.

use std::collections::HashSet;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, Utc};
use sqlx::PgPool;

use crate::modules::backup::{metadata, models::BackupSettings, repository};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i32,
    pub at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    pub daily: usize,
    pub weekly: usize,
    pub monthly: usize,
}

fn keep_buckets<K: Eq + Hash>(
    newest_first: &[&Entry],
    limit: usize,
    key: impl Fn(&Entry) -> K,
    keep: &mut HashSet<i32>,
) {
    let mut seen = HashSet::new();
    for entry in newest_first {
        if seen.len() >= limit {
            break;
        }
        if seen.insert(key(entry)) {
            keep.insert(entry.id);
        }
    }
}

pub const PRE_MIN_KEEP: usize = 2;
pub const PRE_MAX_AGE_DAYS: i64 = 30;

pub fn ids_to_delete(entries: &[Entry], policy: Policy) -> Vec<i32> {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.at));

    let mut keep: HashSet<i32> = HashSet::new();
    if let Some(newest) = sorted.first() {
        keep.insert(newest.id);
    }
    keep_buckets(&sorted, policy.daily, |e| e.at.date(), &mut keep);
    keep_buckets(
        &sorted,
        policy.weekly,
        |e| {
            let week = e.at.date().iso_week();
            (week.year(), week.week())
        },
        &mut keep,
    );
    keep_buckets(
        &sorted,
        policy.monthly,
        |e| (e.at.date().year(), e.at.date().month()),
        &mut keep,
    );

    sorted
        .iter()
        .filter(|e| !keep.contains(&e.id))
        .map(|e| e.id)
        .collect()
}

pub fn pre_ids_to_delete(entries: &[(i32, DateTime<Utc>)], now: DateTime<Utc>) -> Vec<i32> {
    let mut sorted: Vec<&(i32, DateTime<Utc>)> = entries.iter().collect();
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));

    sorted
        .into_iter()
        .enumerate()
        .filter(|(index, entry)| {
            *index >= PRE_MIN_KEEP && now - entry.1 > Duration::days(PRE_MAX_AGE_DAYS)
        })
        .map(|(_, entry)| entry.0)
        .collect()
}

/// Borra el archivo (y su .meta.json) y marca la fila. Devuelve cuántos borró.
pub async fn apply(
    pool: &PgPool,
    settings: &BackupSettings,
    backup_dir: &Path,
) -> Result<usize, sqlx::Error> {
    let candidates = repository::list_rotatable(pool).await?;
    let entries: Vec<Entry> = candidates
        .iter()
        .map(|r| Entry {
            id: r.id,
            at: r.started_at.with_timezone(&Local).naive_local(),
        })
        .collect();

    let policy = Policy {
        daily: settings.keep_daily.max(1) as usize,
        weekly: settings.keep_weekly.max(0) as usize,
        monthly: settings.keep_monthly.max(0) as usize,
    };

    let mut removed = 0;
    for id in ids_to_delete(&entries, policy) {
        let Some(record) = candidates.iter().find(|r| r.id == id) else {
            continue;
        };
        if delete_files(record.file_name.as_deref(), backup_dir).await {
            repository::mark_file_deleted(pool, id).await?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub async fn apply_pre(pool: &PgPool, backup_dir: &Path) -> Result<usize, sqlx::Error> {
    let candidates = repository::list_pre_rotatable(pool).await?;
    let entries: Vec<(i32, DateTime<Utc>)> = candidates
        .iter()
        .map(|record| (record.id, record.started_at))
        .collect();

    let mut removed = 0;
    for id in pre_ids_to_delete(&entries, Utc::now()) {
        let Some(record) = candidates.iter().find(|candidate| candidate.id == id) else {
            continue;
        };
        if delete_files(record.file_name.as_deref(), backup_dir).await {
            repository::mark_file_deleted(pool, id).await?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn managed_backup_path(backup_dir: &Path, name: &str) -> Option<PathBuf> {
    if !(name.starts_with("cashflow_") && (name.ends_with(".dump") || name.ends_with(".dump.enc")))
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        || Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name)
    {
        return None;
    }
    Some(backup_dir.join(name))
}

async fn delete_files(name: Option<&str>, backup_dir: &Path) -> bool {
    let Some(name) = name else {
        return true;
    };
    let Some(path) = managed_backup_path(backup_dir, name) else {
        tracing::warn!(file = name, "Rotación: nombre inesperado, no se borra");
        return false;
    };
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(file = %path.display(), error = %e, "Rotación: no se pudo borrar");
            return false;
        }
    }
    let _ = tokio::fs::remove_file(metadata::meta_path(&path)).await;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(month: u32, day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, month, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
    }

    fn ago(days: i64) -> DateTime<Utc> {
        Utc::now() - Duration::days(days)
    }

    fn entry(id: i32, month: u32, day: u32, hour: u32) -> Entry {
        Entry {
            id,
            at: at(month, day, hour),
        }
    }

    #[test]
    fn keeps_last_n_days_only() {
        let entries: Vec<Entry> = (1..=10).map(|d| entry(d as i32, 10, d, 21)).collect();
        let doomed = ids_to_delete(
            &entries,
            Policy {
                daily: 3,
                weekly: 0,
                monthly: 0,
            },
        );
        assert_eq!(doomed.len(), 7);
        assert!(!doomed.contains(&10) && !doomed.contains(&9) && !doomed.contains(&8));
    }

    #[test]
    fn keeps_latest_of_each_day() {
        let entries = vec![entry(1, 10, 8, 9), entry(2, 10, 8, 21), entry(3, 10, 9, 21)];
        let doomed = ids_to_delete(
            &entries,
            Policy {
                daily: 2,
                weekly: 0,
                monthly: 0,
            },
        );
        assert_eq!(doomed, vec![1]);
    }

    #[test]
    fn monthly_policy_preserves_old_backups() {
        let entries = vec![
            entry(1, 6, 15, 21),
            entry(2, 7, 15, 21),
            entry(3, 10, 12, 21),
            entry(4, 10, 13, 21),
        ];
        let doomed = ids_to_delete(
            &entries,
            Policy {
                daily: 1,
                weekly: 0,
                monthly: 6,
            },
        );
        assert_eq!(doomed, vec![3]);
    }

    #[test]
    fn newest_is_never_deleted() {
        let entries = vec![entry(1, 10, 1, 9), entry(2, 10, 2, 9)];
        let doomed = ids_to_delete(
            &entries,
            Policy {
                daily: 1,
                weekly: 0,
                monthly: 0,
            },
        );
        assert!(!doomed.contains(&2));
    }

    #[test]
    fn managed_backup_path_uses_only_a_validated_file_name() {
        let dir = Path::new(r"C:\CashFlow-Backups");
        assert_eq!(
            managed_backup_path(dir, "cashflow_2026-10-15_120000.dump"),
            Some(dir.join("cashflow_2026-10-15_120000.dump")),
        );
        assert!(managed_backup_path(dir, r"cashflow_..\outside.dump").is_none());
        assert!(managed_backup_path(dir, r"..\outside.dump").is_none());
        assert!(managed_backup_path(dir, "report.txt").is_none());
    }

    #[test]
    fn pre_backups_keep_two_newest_and_recent_ones() {
        let entries = vec![(1, ago(60)), (2, ago(40)), (3, ago(5)), (4, ago(1))];
        let mut doomed = pre_ids_to_delete(&entries, Utc::now());
        doomed.sort();
        assert_eq!(doomed, vec![1, 2]);
    }

    #[test]
    fn pre_backups_never_drop_below_two() {
        let entries = vec![(1, ago(90)), (2, ago(60))];
        assert!(pre_ids_to_delete(&entries, Utc::now()).is_empty());
    }

    #[test]
    fn recent_pre_backups_are_kept_even_if_many() {
        let entries = vec![(1, ago(1)), (2, ago(2)), (3, ago(3)), (4, ago(4))];
        assert!(pre_ids_to_delete(&entries, Utc::now()).is_empty());
    }
}

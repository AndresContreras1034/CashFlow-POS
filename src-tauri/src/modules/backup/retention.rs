//! Rotación: por cada bucket (día, semana ISO, mes) se conserva el backup más
//! reciente, para los N buckets más recientes que tengan backups.
//! El resultado es la unión de las tres políticas; el más nuevo nunca se borra.

use std::collections::HashSet;
use std::hash::Hash;
use std::path::PathBuf;

use chrono::{Datelike, Local, NaiveDateTime};
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

pub fn ids_to_delete(entries: &[Entry], policy: Policy) -> Vec<i32> {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| b.at.cmp(&a.at));

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

/// Borra el archivo (y su .meta.json) y marca la fila. Devuelve cuántos borró.
pub async fn apply(pool: &PgPool, settings: &BackupSettings) -> Result<usize, sqlx::Error> {
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
        if delete_files(record.file_name.as_deref(), record.file_path.as_deref()).await {
            repository::mark_file_deleted(pool, id).await?;
            removed += 1;
        }
    }
    Ok(removed)
}

async fn delete_files(name: Option<&str>, path: Option<&str>) -> bool {
    let (Some(name), Some(path)) = (name, path) else {
        return true;
    };
    // Defensa: solo se borran archivos con el patrón que genera este módulo.
    if !(name.starts_with("cashflow_") && (name.ends_with(".dump") || name.ends_with(".dump.enc"))) {
        tracing::warn!(file = name, "Rotación: nombre inesperado, no se borra");
        return false;
    }
    let path = PathBuf::from(path);
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

    fn entry(id: i32, month: u32, day: u32, hour: u32) -> Entry {
        Entry {
            id,
            at: at(month, day, hour),
        }
    }

    #[test]
    fn keeps_last_n_days_only() {
        let entries: Vec<Entry> = (1..=10).map(|d| entry(d as i32, 10, d, 21)).collect();
        let doomed = ids_to_delete(&entries, Policy { daily: 3, weekly: 0, monthly: 0 });
        assert_eq!(doomed.len(), 7);
        assert!(!doomed.contains(&10) && !doomed.contains(&9) && !doomed.contains(&8));
    }

    #[test]
    fn keeps_latest_of_each_day() {
        let entries = vec![entry(1, 10, 8, 9), entry(2, 10, 8, 21), entry(3, 10, 9, 21)];
        let doomed = ids_to_delete(&entries, Policy { daily: 2, weekly: 0, monthly: 0 });
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
        let doomed = ids_to_delete(&entries, Policy { daily: 1, weekly: 0, monthly: 6 });
        assert_eq!(doomed, vec![3]);
    }

    #[test]
    fn newest_is_never_deleted() {
        let entries = vec![entry(1, 10, 1, 9), entry(2, 10, 2, 9)];
        let doomed = ids_to_delete(&entries, Policy { daily: 1, weekly: 0, monthly: 0 });
        assert!(!doomed.contains(&2));
    }
}

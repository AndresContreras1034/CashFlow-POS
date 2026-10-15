use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const STALE_AFTER: Duration = Duration::from_secs(60 * 60);
const TEMP_PREFIXES: [&str; 2] = ["cashflow_backup_", "cashflow_restore_"];

fn should_remove(name: &str, modified: SystemTime, now: SystemTime, prefixes: &[&str]) -> bool {
    name.ends_with(".tmp")
        && prefixes.iter().any(|prefix| name.starts_with(prefix))
        && now
            .duration_since(modified)
            .is_ok_and(|age| age > STALE_AFTER)
}

pub fn directory() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "No está definida la carpeta LOCALAPPDATA".to_string())?;

    #[cfg(not(windows))]
    let root = std::env::temp_dir();

    Ok(root.join("CashFlow").join("tmp"))
}

pub async fn ensure_directory() -> Result<PathBuf, String> {
    let directory = directory()?;
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| {
            format!(
                "No se pudo crear la carpeta temporal segura ({}): {error}",
                directory.display()
            )
        })?;
    Ok(directory)
}

pub async fn cleanup_stale() -> Result<(), String> {
    let directory = ensure_directory().await?;
    remove_stale(&directory, &TEMP_PREFIXES).await
}

pub async fn remove_stale(directory: &std::path::Path, prefixes: &[&str]) -> Result<(), String> {
    let mut entries = match tokio::fs::read_dir(directory).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "No se pudo revisar la carpeta temporal {}: {error}",
                directory.display()
            ));
        }
    };
    let now = SystemTime::now();

    while let Some(entry) = entries.next_entry().await.map_err(|error| {
        format!(
            "No se pudo revisar la carpeta temporal {}: {error}",
            directory.display()
        )
    })? {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !name.ends_with(".tmp") || !prefixes.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }
        let metadata = entry.metadata().await.map_err(|error| {
            format!(
                "No se pudieron consultar los metadatos del temporal {}: {error}",
                entry.path().display()
            )
        })?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().map_err(|error| {
            format!(
                "No se pudo consultar la fecha del temporal {}: {error}",
                entry.path().display()
            )
        })?;
        if !should_remove(&name, modified, now, prefixes) {
            continue;
        }
        tokio::fs::remove_file(entry.path())
            .await
            .map_err(|error| {
                format!(
                    "No se pudo eliminar el temporal obsoleto {}: {error}",
                    entry.path().display()
                )
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn cleanup_preserves_recent_and_unrelated_temporaries() {
        let directory = std::env::temp_dir().join(format!("cashflow_temp_{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&directory).await.unwrap();
        let backup = directory.join("cashflow_backup_test.tmp");
        let unrelated = directory.join("other_test.tmp");
        tokio::fs::write(&backup, b"backup").await.unwrap();
        tokio::fs::write(&unrelated, b"unrelated").await.unwrap();

        remove_stale(&directory, &["cashflow_backup_"])
            .await
            .unwrap();

        assert!(backup.exists());
        assert!(unrelated.exists());
        tokio::fs::remove_dir_all(directory).await.unwrap();
    }

    #[test]
    fn stale_cleanup_requires_age_and_a_known_prefix() {
        let now = SystemTime::now();
        let old = now - STALE_AFTER - Duration::from_secs(1);
        assert!(should_remove(
            "cashflow_backup_old.tmp",
            old,
            now,
            &["cashflow_backup_"]
        ));
        assert!(!should_remove(
            "cashflow_backup_recent.tmp",
            now,
            now,
            &["cashflow_backup_"]
        ));
        assert!(!should_remove(
            "unrelated_old.tmp",
            old,
            now,
            &["cashflow_backup_"]
        ));
    }
}

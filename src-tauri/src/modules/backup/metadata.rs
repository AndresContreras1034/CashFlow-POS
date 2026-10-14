//! Hash del archivo y metadatos que viajan junto a cada backup.
//! La restauración (fase 4) los usará para rechazar backups incompatibles.

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: DateTime<Utc>,
    pub kind: String,
    pub database: String,
    pub pg_server_version: String,
    pub pg_dump_version: String,
    pub last_migration: i64,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub encrypted: bool,
}

/// `cashflow_X.dump` -> `cashflow_X.dump.meta.json`
pub fn meta_path(dump: &Path) -> PathBuf {
    let mut name = dump.as_os_str().to_os_string();
    name.push(".meta.json");
    PathBuf::from(name)
}

pub async fn sha256_file(path: PathBuf) -> Result<String, String> {
    tokio::task::spawn_blocking(move || -> Result<String, std::io::Error> {
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1 << 20];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    })
    .await
    .map_err(|e| format!("No se pudo calcular el hash: {e}"))?
    .map_err(|e| format!("No se pudo calcular el hash: {e}"))
}

/// Escritura atómica: primero `.tmp`, luego renombrar.
pub async fn write(dump: &Path, meta: &BackupMetadata) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(meta).map_err(|e| e.to_string())?;
    let target = meta_path(dump);
    let mut tmp_name = target.as_os_str().to_os_string();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);

    tokio::fs::write(&tmp, json)
        .await
        .map_err(|e| format!("No se pudo escribir los metadatos: {e}"))?;
    tokio::fs::rename(&tmp, &target)
        .await
        .map_err(|e| format!("No se pudo finalizar los metadatos: {e}"))
}

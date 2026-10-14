use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Manual,
    Scheduled,
    CatchUp,
    PreRestore,
    PreImport,
}

impl BackupKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Scheduled => "scheduled",
            Self::CatchUp => "catchup",
            Self::PreRestore => "pre_restore",
            Self::PreImport => "pre_import",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupSettings {
    pub id: i16,
    /// None = carpeta por defecto (Documentos\CashFlow-Backups).
    pub local_dir: Option<String>,
    /// None = detección automática de pg_dump.
    pub pg_bin_dir: Option<String>,
    pub keep_daily: i32,
    pub keep_weekly: i32,
    pub keep_monthly: i32,
    pub auto_enabled: bool,
    /// Hora local del PC.
    pub schedule_time: NaiveTime,
    /// Máscara de bits: lunes = 1 ... domingo = 64.
    pub schedule_days: i16,
    pub warn_after_days: i32,
    pub critical_after_days: i32,
    pub encrypt_enabled: bool,
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: i32,
    /// "tls" (465) o "starttls" (587)
    pub smtp_security: String,
    pub smtp_username: Option<String>,
    pub smtp_from_name: Option<String>,
    pub email_on_failure: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupRecord {
    pub id: i32,
    pub kind: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub file_name: Option<String>,
    pub file_path: Option<String>,
    pub size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub pg_server_version: Option<String>,
    pub last_migration: Option<i64>,
    pub error_message: Option<String>,
    pub created_by: String,
    pub file_deleted_at: Option<DateTime<Utc>>,
    pub encrypted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupRecipient {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub active: bool,
    pub send_attachment: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupDelivery {
    pub id: i32,
    pub backup_id: Option<i32>,
    /// "success" | "failure" | "test"
    pub notice: String,
    pub recipient_email: String,
    pub attached: bool,
    /// "sent" | "failed"
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

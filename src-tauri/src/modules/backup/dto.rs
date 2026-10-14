use serde::{Deserialize, Serialize};

// Misma semántica que UpdateSettingsDto:
// - keep_*: None = no cambiar.
// - local_dir y pg_bin_dir se reemplazan siempre; None = volver al valor por
//   defecto. El frontend debe enviar el objeto completo.
#[derive(Debug, Deserialize)]
pub struct UpdateBackupSettingsDto {
    pub local_dir: Option<String>,
    pub pg_bin_dir: Option<String>,
    pub keep_daily: Option<i32>,
    pub keep_weekly: Option<i32>,
    pub keep_monthly: Option<i32>,
    pub auto_enabled: Option<bool>,
    /// "HH:MM"
    pub schedule_time: Option<String>,
    pub schedule_days: Option<i16>,
    pub warn_after_days: Option<i32>,
    pub critical_after_days: Option<i32>,
    pub encrypt_enabled: Option<bool>,
    pub smtp_enabled: Option<bool>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_security: Option<String>,
    // Nullable: se reemplazan siempre (None = borrar).
    pub smtp_username: Option<String>,
    pub smtp_from_name: Option<String>,
    pub email_on_failure: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipientDto {
    pub email: String,
    pub name: Option<String>,
    pub send_attachment: Option<bool>,
}

// name se reemplaza siempre; el correo no se edita (se borra y se añade otro).
#[derive(Debug, Deserialize)]
pub struct UpdateRecipientDto {
    pub name: Option<String>,
    pub active: Option<bool>,
    pub send_attachment: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SecretsStatus {
    pub encryption_password_set: bool,
    pub smtp_password_set: bool,
}

#[derive(Debug, Serialize)]
pub struct EmailTestResult {
    pub email: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Diagnóstico para la pantalla de Ajustes: ¿puede funcionar el backup aquí?
#[derive(Debug, Serialize)]
pub struct BackupEnvironment {
    pub database: String,
    pub server_version: String,
    pub database_size_bytes: i64,
    pub default_dir: String,
    pub effective_dir: String,
    pub dir_writable: bool,
    pub pg_dump_path: Option<String>,
    pub pg_dump_version: Option<String>,
    /// Motivo por el que no se encontró un pg_dump utilizable.
    pub tools_error: Option<String>,
}

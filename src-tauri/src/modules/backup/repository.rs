use chrono::{DateTime, NaiveTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::modules::backup::{
    dto::UpdateBackupSettingsDto,
    models::{BackupDelivery, BackupKind, BackupRecipient, BackupRecord, BackupSettings},
};

pub struct DbInfo {
    pub server_version: String,
    pub server_version_num: i32,
    pub database: String,
    pub size_bytes: i64,
}

impl DbInfo {
    /// 180004 -> 18
    pub fn major(&self) -> u32 {
        (self.server_version_num / 10_000).max(0) as u32
    }
}

pub struct SuccessData {
    pub kind: BackupKind,
    pub started_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub pg_server_version: String,
    pub last_migration: i64,
    pub created_by: String,
    pub encrypted: bool,
}

pub async fn get_settings(pool: &PgPool) -> Result<Option<BackupSettings>, sqlx::Error> {
    sqlx::query_as::<_, BackupSettings>("SELECT * FROM backup_settings WHERE id = 1")
        .fetch_optional(pool)
        .await
}

pub async fn update_settings(
    pool: &PgPool,
    dto: UpdateBackupSettingsDto,
    schedule_time: Option<NaiveTime>,
) -> Result<Option<BackupSettings>, sqlx::Error> {
    sqlx::query_as::<_, BackupSettings>(
        "UPDATE backup_settings
         SET local_dir           = $1,
             pg_bin_dir          = $2,
             keep_daily          = COALESCE($3, keep_daily),
             keep_weekly         = COALESCE($4, keep_weekly),
             keep_monthly        = COALESCE($5, keep_monthly),
             auto_enabled        = COALESCE($6, auto_enabled),
             schedule_time       = COALESCE($7, schedule_time),
             schedule_days       = COALESCE($8, schedule_days),
             warn_after_days     = COALESCE($9, warn_after_days),
             critical_after_days = COALESCE($10, critical_after_days),
             encrypt_enabled     = COALESCE($11, encrypt_enabled),
             smtp_enabled        = COALESCE($12, smtp_enabled),
             smtp_host           = COALESCE($13, smtp_host),
             smtp_port           = COALESCE($14, smtp_port),
             smtp_security       = COALESCE($15, smtp_security),
             smtp_username       = $16,
             smtp_from_name      = $17,
             email_on_failure    = COALESCE($18, email_on_failure)
         WHERE id = 1
         RETURNING *",
    )
    .bind(dto.local_dir)
    .bind(dto.pg_bin_dir)
    .bind(dto.keep_daily)
    .bind(dto.keep_weekly)
    .bind(dto.keep_monthly)
    .bind(dto.auto_enabled)
    .bind(schedule_time)
    .bind(dto.schedule_days)
    .bind(dto.warn_after_days)
    .bind(dto.critical_after_days)
    .bind(dto.encrypt_enabled)
    .bind(dto.smtp_enabled)
    .bind(dto.smtp_host)
    .bind(dto.smtp_port)
    .bind(dto.smtp_security)
    .bind(dto.smtp_username)
    .bind(dto.smtp_from_name)
    .bind(dto.email_on_failure)
    .fetch_optional(pool)
    .await
}

#[derive(Debug, FromRow)]
pub struct HistorySummary {
    pub last_success: Option<DateTime<Utc>>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_attempt_status: Option<String>,
    pub last_error: Option<String>,
    pub failures_since_success: i64,
}

/// Una sola consulta con todo lo que necesitan el estado y el scheduler.
pub async fn history_summary(pool: &PgPool) -> Result<HistorySummary, sqlx::Error> {
    sqlx::query_as::<_, HistorySummary>(
        "WITH ok AS (
             SELECT MAX(finished_at) AS at FROM backup_history WHERE status = 'success'
         ),
         latest AS (
             SELECT status, finished_at, error_message
             FROM backup_history
             ORDER BY finished_at DESC, id DESC
             LIMIT 1
         )
         SELECT (SELECT at FROM ok)                  AS last_success,
                (SELECT finished_at FROM latest)     AS last_attempt_at,
                (SELECT status FROM latest)          AS last_attempt_status,
                (SELECT error_message FROM latest)   AS last_error,
                (SELECT COUNT(*) FROM backup_history
                  WHERE status = 'failed'
                    AND finished_at > COALESCE((SELECT at FROM ok), '-infinity'::timestamptz)
                ) AS failures_since_success",
    )
    .fetch_one(pool)
    .await
}

pub async fn insert_success(pool: &PgPool, data: SuccessData) -> Result<BackupRecord, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "INSERT INTO backup_history
            (kind, status, started_at, duration_ms, file_name, file_path,
             size_bytes, sha256, pg_server_version, last_migration, created_by, encrypted)
         VALUES ($1, 'success', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING *",
    )
    .bind(data.kind.as_str())
    .bind(data.started_at)
    .bind(data.duration_ms)
    .bind(data.file_name)
    .bind(data.file_path)
    .bind(data.size_bytes)
    .bind(data.sha256)
    .bind(data.pg_server_version)
    .bind(data.last_migration)
    .bind(data.created_by)
    .bind(data.encrypted)
    .fetch_one(pool)
    .await
}

pub async fn insert_failure(
    pool: &PgPool,
    kind: BackupKind,
    started_at: DateTime<Utc>,
    duration_ms: i64,
    error_message: &str,
    created_by: &str,
) -> Result<BackupRecord, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "INSERT INTO backup_history
            (kind, status, started_at, duration_ms, error_message, created_by)
         VALUES ($1, 'failed', $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(kind.as_str())
    .bind(started_at)
    .bind(duration_ms)
    .bind(error_message)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn list_history(pool: &PgPool, limit: i64) -> Result<Vec<BackupRecord>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "SELECT * FROM backup_history ORDER BY started_at DESC, id DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Backups con archivo vigente que la rotación puede borrar.
/// Los `pre_*` quedan fuera: su política se decide en la fase de restauración.
pub async fn list_rotatable(pool: &PgPool) -> Result<Vec<BackupRecord>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "SELECT * FROM backup_history
         WHERE status = 'success'
           AND file_deleted_at IS NULL
           AND kind IN ('manual', 'scheduled', 'catchup')
         ORDER BY started_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await
}

/// Backups previos a una restauración o importación, con archivo vigente.
pub async fn list_pre_rotatable(pool: &PgPool) -> Result<Vec<BackupRecord>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "SELECT * FROM backup_history
         WHERE status = 'success'
           AND file_deleted_at IS NULL
           AND kind IN ('pre_restore', 'pre_import')
         ORDER BY started_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_file_deleted(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE backup_history SET file_deleted_at = NOW()
         WHERE id = $1 AND file_deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn db_info(pool: &PgPool) -> Result<DbInfo, sqlx::Error> {
    let (server_version, server_version_num, database, size_bytes): (String, i32, String, i64) =
        sqlx::query_as(
            "SELECT current_setting('server_version'),
                    current_setting('server_version_num')::int,
                    current_database()::text,
                    pg_database_size(current_database())",
        )
        .fetch_one(pool)
        .await?;
    Ok(DbInfo {
        server_version,
        server_version_num,
        database,
        size_bytes,
    })
}

/// Última migración de sqlx aplicada (para los metadatos del backup).
pub async fn last_migration(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(version), 0::bigint) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}

pub async fn business_name(pool: &PgPool) -> Result<String, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT business_name FROM app_settings WHERE id = 1")
            .fetch_optional(pool)
            .await?
            .unwrap_or_else(|| "CashFlow POS".to_string()),
    )
}

// ---------------- Destinatarios ----------------

pub async fn list_recipients(pool: &PgPool) -> Result<Vec<BackupRecipient>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecipient>("SELECT * FROM backup_recipients ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn active_recipients(pool: &PgPool) -> Result<Vec<BackupRecipient>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecipient>("SELECT * FROM backup_recipients WHERE active ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn insert_recipient(
    pool: &PgPool,
    email: &str,
    name: Option<&str>,
    send_attachment: bool,
) -> Result<BackupRecipient, sqlx::Error> {
    sqlx::query_as::<_, BackupRecipient>(
        "INSERT INTO backup_recipients (email, name, send_attachment)
         VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(email)
    .bind(name)
    .bind(send_attachment)
    .fetch_one(pool)
    .await
}

pub async fn update_recipient(
    pool: &PgPool,
    id: i32,
    name: Option<&str>,
    active: Option<bool>,
    send_attachment: Option<bool>,
) -> Result<Option<BackupRecipient>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecipient>(
        "UPDATE backup_recipients
         SET name            = $2,
             active          = COALESCE($3, active),
             send_attachment = COALESCE($4, send_attachment)
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(active)
    .bind(send_attachment)
    .fetch_optional(pool)
    .await
}

/// Devuelve el correo borrado, o None si no existía.
pub async fn delete_recipient(pool: &PgPool, id: i32) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("DELETE FROM backup_recipients WHERE id = $1 RETURNING email")
        .bind(id)
        .fetch_optional(pool)
        .await
}

// ---------------- Envíos ----------------

pub async fn insert_delivery(
    pool: &PgPool,
    backup_id: Option<i32>,
    notice: &str,
    email: &str,
    attached: bool,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO backup_deliveries
            (backup_id, notice, recipient_email, attached, status, error_message)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(backup_id)
    .bind(notice)
    .bind(email)
    .bind(attached)
    .bind(if error.is_some() { "failed" } else { "sent" })
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_deliveries(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<BackupDelivery>, sqlx::Error> {
    sqlx::query_as::<_, BackupDelivery>(
        "SELECT * FROM backup_deliveries ORDER BY created_at DESC, id DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Último backup exitoso con archivo vigente, de las últimas `hours` horas.
pub async fn latest_success_within(
    pool: &PgPool,
    hours: i32,
) -> Result<Option<BackupRecord>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecord>(
        "SELECT * FROM backup_history
         WHERE status = 'success'
           AND file_deleted_at IS NULL
           AND finished_at > NOW() - make_interval(hours => $1)
         ORDER BY finished_at DESC, id DESC
         LIMIT 1",
    )
    .bind(hours)
    .fetch_optional(pool)
    .await
}

/// Destinatarios activos a los que ya se intentó enviar este backup (1 a 3 veces),
/// sin ningún envío correcto y con el último intento de hace más de 30 minutos.
/// Un destinatario añadido después del backup (0 intentos) no entra.
pub async fn recipients_needing_retry(
    pool: &PgPool,
    backup_id: i32,
) -> Result<Vec<BackupRecipient>, sqlx::Error> {
    sqlx::query_as::<_, BackupRecipient>(
        "SELECT r.* FROM backup_recipients r
         WHERE r.active
           AND NOT EXISTS (
                SELECT 1 FROM backup_deliveries d
                WHERE d.backup_id = $1 AND d.notice = 'success'
                  AND d.recipient_email = r.email AND d.status = 'sent')
           AND (SELECT COUNT(*) FROM backup_deliveries d
                WHERE d.backup_id = $1 AND d.notice = 'success'
                  AND d.recipient_email = r.email) BETWEEN 1 AND 3
           AND (SELECT MAX(d.created_at) FROM backup_deliveries d
                WHERE d.backup_id = $1 AND d.notice = 'success'
                  AND d.recipient_email = r.email) < NOW() - INTERVAL '30 minutes'",
    )
    .bind(backup_id)
    .fetch_all(pool)
    .await
}

pub async fn failure_notice_recent(pool: &PgPool, hours: i32) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM backup_deliveries
            WHERE notice = 'failure'
              AND created_at > NOW() - make_interval(hours => $1))",
    )
    .bind(hours)
    .fetch_one(pool)
    .await
}

//! Registro de backups en auditoría.
//! Un fallo al auditar nunca debe invalidar el backup: solo se avisa en el log.
//! Se usa una conexión aparte (sin transacción), así el evento de fallo
//! sobrevive aunque algo haga rollback.

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::audit::{
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::backup::models::{BackupKind, BackupRecord};

fn kind_label(kind: BackupKind) -> &'static str {
    match kind {
        BackupKind::Manual => "manual",
        BackupKind::Scheduled => "programado",
        BackupKind::CatchUp => "recuperado al arrancar",
        BackupKind::PreRestore => "previo a restauración",
        BackupKind::PreImport => "previo a importación",
    }
}

pub fn format_size(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes.max(0) as f64;
    if b < KB {
        format!("{} B", bytes.max(0))
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

async fn write(pool: &PgPool, event: NewAuditEvent) {
    match pool.acquire().await {
        Ok(mut conn) => {
            if let Err(e) = audit_repo::insert_event(&mut conn, event).await {
                tracing::warn!(error = %e, "No se pudo registrar el backup en auditoría");
            }
        }
        Err(e) => tracing::warn!(error = %e, "No se pudo registrar el backup en auditoría"),
    }
}

pub async fn success(
    pool: &PgPool,
    kind: BackupKind,
    record: &BackupRecord,
    actor: Option<String>,
) {
    let file = record.file_name.clone().unwrap_or_default();
    let size = record.size_bytes.unwrap_or(0);
    write(
        pool,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Backup,
            action: "create".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("backup".to_string()),
            entity_id: Some(record.id.to_string()),
            summary: format!(
                "Backup {} creado: {} ({})",
                kind_label(kind),
                file,
                format_size(size)
            ),
            changes: None,
            metadata: Some(json!({
                "kind": kind.as_str(),
                "file_name": file,
                "size_bytes": size,
                "sha256": record.sha256,
                "duration_ms": record.duration_ms,
            })),
            error_message: None,
        },
    )
    .await;
}

pub async fn failure(pool: &PgPool, kind: BackupKind, message: &str, actor: Option<String>) {
    write(
        pool,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Error,
            module: AuditModule::Backup,
            action: "create".to_string(),
            outcome: AuditOutcome::Failure,
            actor,
            entity_type: Some("backup".to_string()),
            entity_id: None,
            summary: format!("Backup {} fallido", kind_label(kind)),
            changes: None,
            metadata: Some(json!({ "kind": kind.as_str() })),
            error_message: Some(message.to_string()),
        },
    )
    .await;
}

pub async fn restore_failure(pool: &PgPool, message: &str, actor: Option<String>) {
    write(
        pool,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Error,
            module: AuditModule::Backup,
            action: "restore".to_string(),
            outcome: AuditOutcome::Failure,
            actor,
            entity_type: Some("backup".to_string()),
            entity_id: None,
            summary: "Restauración de backup fallida".to_string(),
            changes: None,
            metadata: None,
            error_message: Some(message.to_string()),
        },
    )
    .await;
}

pub async fn config_change(pool: &PgPool, summary: String, actor: Option<String>) {
    write(
        pool,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Backup,
            action: "update".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("backup_settings".to_string()),
            entity_id: Some("1".to_string()),
            summary,
            changes: None,
            metadata: None,
            error_message: None,
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1_134_322), "1.1 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}

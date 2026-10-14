//! Correos de backup: aviso de éxito (con adjunto cifrado si cabe), aviso de
//! fallo (limitado a uno cada 12 h), reintentos y correo de prueba.
//! Todo corre en segundo plano; un fallo aquí nunca afecta al backup.

use chrono::Local;
use lettre::message::Mailbox;
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::modules::backup::{
    destinations::smtp::{self, SmtpConfig, Transport, MAX_ATTACHMENT_BYTES},
    dto::EmailTestResult,
    events::format_size,
    models::{BackupRecipient, BackupRecord, BackupSettings},
    repository,
    secrets::{self, Secret},
    service,
};

/// Evita que un reintento y el envío normal se pisen.
static EMAIL_LOCK: Mutex<()> = Mutex::const_new(());

const FAILURE_NOTICE_COOLDOWN_HOURS: i32 = 12;
const RETRY_WINDOW_HOURS: i32 = 24;

struct Ctx {
    transport: Transport,
    from: Mailbox,
    business: String,
}

async fn build_ctx(pool: &PgPool, settings: &BackupSettings) -> Result<Ctx, String> {
    let username = settings
        .smtp_username
        .clone()
        .filter(|u| !u.is_empty())
        .ok_or("Falta el correo remitente (la cuenta de Gmail)")?;
    let password = secrets::get(Secret::SmtpPassword)?
        .ok_or("Falta la contraseña de aplicación del correo")?;
    let port = u16::try_from(settings.smtp_port).map_err(|_| "Puerto SMTP no válido".to_string())?;

    let transport = smtp::transport(&SmtpConfig {
        host: settings.smtp_host.clone(),
        port,
        starttls: settings.smtp_security == "starttls",
        username: username.clone(),
        password,
    })?;
    let from = smtp::mailbox(settings.smtp_from_name.as_deref(), &username)?;
    let business = repository::business_name(pool)
        .await
        .unwrap_or_else(|_| "CashFlow POS".to_string());

    Ok(Ctx { transport, from, business })
}

async fn record_failed(
    pool: &PgPool,
    backup_id: Option<i32>,
    notice: &str,
    recipients: &[BackupRecipient],
    error: &str,
) {
    for r in recipients {
        let _ = repository::insert_delivery(pool, backup_id, notice, &r.email, false, Some(error)).await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn deliver(
    pool: &PgPool,
    ctx: &Ctx,
    backup_id: Option<i32>,
    notice: &str,
    to: &BackupRecipient,
    subject: &str,
    body: &str,
    attachment: Option<(&str, &[u8])>,
) -> Result<(), String> {
    let result = smtp::send(&ctx.transport, &ctx.from, &to.email, subject, body, attachment).await;
    if let Err(e) = repository::insert_delivery(
        pool,
        backup_id,
        notice,
        &to.email,
        attachment.is_some(),
        result.as_ref().err().map(String::as_str),
    )
    .await
    {
        tracing::warn!(error = %e, "No se pudo registrar el envío del correo");
    }
    result
}

fn attach_note(attached: bool, wants_file: bool, encrypted: bool, size: i64) -> String {
    if attached {
        "El backup cifrado va adjunto a este correo.".to_string()
    } else if !wants_file {
        "Este destinatario recibe solo el aviso, sin el archivo.".to_string()
    } else if !encrypted {
        "El archivo NO se adjunta porque el cifrado está desactivado (por seguridad nunca se \
         envían datos sin cifrar por correo). Activa el cifrado en Ajustes → Backup."
            .to_string()
    } else if size as u64 > MAX_ATTACHMENT_BYTES {
        format!(
            "El archivo NO se adjunta porque pesa {} y el máximo por correo es {}.",
            format_size(size),
            format_size(MAX_ATTACHMENT_BYTES as i64)
        )
    } else {
        "No se pudo adjuntar el archivo (ya no está en la carpeta o no se pudo leer).".to_string()
    }
}

async fn send_success(
    pool: &PgPool,
    settings: &BackupSettings,
    record: &BackupRecord,
    recipients: Vec<BackupRecipient>,
) -> Result<(), String> {
    if recipients.is_empty() {
        return Ok(());
    }
    let ctx = match build_ctx(pool, settings).await {
        Ok(ctx) => ctx,
        Err(e) => {
            record_failed(pool, Some(record.id), "success", &recipients, &e).await;
            return Err(e);
        }
    };

    let file_name = record.file_name.clone().unwrap_or_default();
    let size = record.size_bytes.unwrap_or(0);
    let can_attach = record.encrypted
        && record.file_deleted_at.is_none()
        && (size as u64) <= MAX_ATTACHMENT_BYTES;
    let bytes = if can_attach && recipients.iter().any(|r| r.send_attachment) {
        match tokio::fs::read(record.file_path.as_deref().unwrap_or("")).await {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(error = %e, "No se pudo leer el backup para adjuntarlo");
                None
            }
        }
    } else {
        None
    };

    let when = record.finished_at.with_timezone(&Local).format("%d/%m/%Y %H:%M").to_string();
    let subject = format!("[CashFlow POS] Backup correcto - {} - {when}", ctx.business);
    let location = record
        .file_path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).parent())
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    for r in &recipients {
        let attachment = if r.send_attachment {
            bytes.as_deref().map(|b| (file_name.as_str(), b))
        } else {
            None
        };
        let body = format!(
            "Backup de CashFlow POS completado.\n\n\
             Negocio: {business}\n\
             Fecha: {when}\n\
             Archivo: {file_name}\n\
             Tamaño: {size_text}\n\
             Cifrado: {encrypted}\n\
             SHA-256: {sha}\n\
             Ubicación en el equipo: {location}\n\n\
             {note}\n\n\
             Para restaurarlo necesitarás la contraseña de cifrado (no viaja en este correo).\n",
            business = ctx.business,
            size_text = format_size(size),
            encrypted = if record.encrypted { "Sí (AES-256-GCM)" } else { "No" },
            sha = record.sha256.as_deref().unwrap_or("-"),
            note = attach_note(attachment.is_some(), r.send_attachment, record.encrypted, size),
        );
        let _ = deliver(pool, &ctx, Some(record.id), "success", r, &subject, &body, attachment).await;
    }
    Ok(())
}

pub async fn after_success(pool: &PgPool, record: &BackupRecord) {
    let _guard = EMAIL_LOCK.lock().await;
    let Ok(settings) = service::get_settings(pool).await else { return };
    if !settings.smtp_enabled {
        return;
    }
    let recipients = match repository::active_recipients(pool).await {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, "No se pudieron leer los destinatarios");
            return;
        }
    };
    if let Err(e) = send_success(pool, &settings, record, recipients).await {
        tracing::warn!(error = %e, "No se pudo enviar el correo del backup");
    }
}

pub async fn after_failure(pool: &PgPool, record: &BackupRecord) {
    let _guard = EMAIL_LOCK.lock().await;
    let Ok(settings) = service::get_settings(pool).await else { return };
    if !settings.smtp_enabled || !settings.email_on_failure {
        return;
    }
    if repository::failure_notice_recent(pool, FAILURE_NOTICE_COOLDOWN_HOURS)
        .await
        .unwrap_or(false)
    {
        return;
    }
    let Ok(recipients) = repository::active_recipients(pool).await else { return };
    if recipients.is_empty() {
        return;
    }
    let ctx = match build_ctx(pool, &settings).await {
        Ok(ctx) => ctx,
        Err(e) => {
            record_failed(pool, Some(record.id), "failure", &recipients, &e).await;
            tracing::warn!(error = %e, "No se pudo enviar el aviso de backup fallido");
            return;
        }
    };

    let when = record.finished_at.with_timezone(&Local).format("%d/%m/%Y %H:%M").to_string();
    let subject = format!("[CashFlow POS] Backup FALLIDO - {} - {when}", ctx.business);
    let body = format!(
        "El backup automático de CashFlow POS NO se pudo completar.\n\n\
         Negocio: {}\nFecha: {when}\n\nMotivo:\n{}\n\n\
         Revisa Ajustes → Backup. El sistema reintentará automáticamente; para no saturar \
         tu correo, este aviso se envía como máximo una vez cada {FAILURE_NOTICE_COOLDOWN_HOURS} horas.\n",
        ctx.business,
        record.error_message.as_deref().unwrap_or("(sin detalle)"),
    );
    for r in &recipients {
        let _ = deliver(pool, &ctx, Some(record.id), "failure", r, &subject, &body, None).await;
    }
}

/// Lo llama el scheduler cada minuto; casi siempre sale en la primera consulta.
pub async fn retry_pending(pool: &PgPool) {
    let Ok(settings) = service::get_settings(pool).await else { return };
    if !settings.smtp_enabled {
        return;
    }
    let _guard = EMAIL_LOCK.lock().await;
    let Ok(Some(record)) = repository::latest_success_within(pool, RETRY_WINDOW_HOURS).await else {
        return;
    };
    let Ok(pending) = repository::recipients_needing_retry(pool, record.id).await else { return };
    if pending.is_empty() {
        return;
    }
    tracing::info!(count = pending.len(), "Reintentando correos de backup");
    if let Err(e) = send_success(pool, &settings, &record, pending).await {
        tracing::warn!(error = %e, "Reintento de correo de backup fallido");
    }
}

/// Usa los ajustes GUARDADOS (no los del formulario sin guardar).
pub async fn send_test(pool: &PgPool) -> Result<Vec<EmailTestResult>, String> {
    let _guard = EMAIL_LOCK.lock().await;
    let settings = service::get_settings(pool).await.map_err(|e| e.to_string())?;
    let recipients = repository::active_recipients(pool).await.map_err(|e| e.to_string())?;
    if recipients.is_empty() {
        return Err("Añade al menos un destinatario activo".to_string());
    }
    let ctx = build_ctx(pool, &settings).await?;

    let subject = "[CashFlow POS] Correo de prueba de backups".to_string();
    let body = format!(
        "Este es un correo de prueba de CashFlow POS ({}).\n\
         Si lo estás leyendo, el envío de backups por correo funciona.\n",
        ctx.business
    );

    let mut results = Vec::new();
    for r in &recipients {
        let outcome = deliver(pool, &ctx, None, "test", r, &subject, &body, None).await;
        results.push(EmailTestResult {
            email: r.email.clone(),
            ok: outcome.is_ok(),
            error: outcome.err(),
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_note_explains_why_there_is_no_file() {
        assert!(attach_note(true, true, true, 100).contains("adjunto"));
        assert!(attach_note(false, false, true, 100).contains("solo el aviso"));
        assert!(attach_note(false, true, false, 100).contains("cifrado está desactivado"));
        assert!(attach_note(false, true, true, 30 * 1024 * 1024).contains("máximo"));
    }
}

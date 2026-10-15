use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveTime, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::errors::app_error::AppError;
use crate::modules::backup::{
    crypto,
    dto::{
        BackupEnvironment, CreateRecipientDto, SecretsStatus, UpdateBackupSettingsDto,
        UpdateRecipientDto,
    },
    events,
    metadata::{self, BackupMetadata},
    models::{BackupDelivery, BackupKind, BackupRecipient, BackupRecord, BackupSettings},
    notify,
    pg_tools::{self, ConnParams, PgTools},
    repository::{self, SuccessData},
    retention,
    secrets::{self, Secret},
    temp,
};

const DUMP_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const VERIFY_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const README_TEXT: &str = "\
BACKUPS DE CASHFLOW POS
=======================

Esta carpeta contiene copias de seguridad de la base de datos (PostgreSQL).

  cashflow_AAAA-MM-DD_HHMMSS.dump       Backup sin cifrar
  cashflow_AAAA-MM-DD_HHMMSS.dump.enc   Backup cifrado (AES-256)
  *.meta.json                           Datos informativos del backup

RESTAURAR CON LA APLICACION (lo recomendado)
  Ajustes > Backup > Restaurar desde un backup.
  En un equipo nuevo, en la pantalla de licencia: Restaurar datos desde un backup.
  Si el archivo es .dump.enc necesitas la contrasena de cifrado. SIN ELLA NO SE
  PUEDE RECUPERAR. Guardala fuera de este equipo.

RESTAURAR A MANO (si la aplicacion no arranca)
  Solo para archivos .dump (sin cifrar). Un .dump.enc solo se abre con la aplicacion.
  Desde una consola (CMD), con la ruta de PostgreSQL de tu version:

    \"C:\\Program Files\\PostgreSQL\\18\\bin\\createdb.exe\" -U postgres pos_db_recuperada
    \"C:\\Program Files\\PostgreSQL\\18\\bin\\pg_restore.exe\" -U postgres --no-owner --no-privileges -d pos_db_recuperada \"RUTA\\AL\\ARCHIVO.dump\"

  Despues apunta DATABASE_URL del archivo .env a la base pos_db_recuperada.

Los archivos cashflow_*.dump de esta carpeta los gestiona la aplicacion (rotacion
automatica). No guardes aqui otros archivos con ese nombre.
";

/// Un solo backup a la vez (manual, programado o previo a otra acción).
static BACKUP_LOCK: Mutex<()> = Mutex::const_new(());
static RESTART_PENDING: AtomicBool = AtomicBool::new(false);

pub fn is_running() -> bool {
    BACKUP_LOCK.try_lock().is_err()
}

pub fn restart_pending() -> bool {
    RESTART_PENDING.load(Ordering::SeqCst)
}

pub fn mark_restart_pending() {
    RESTART_PENDING.store(true, Ordering::SeqCst);
}

/// Toma el bloqueo único de backup/restauración.
pub fn acquire() -> Result<tokio::sync::MutexGuard<'static, ()>, AppError> {
    BACKUP_LOCK.try_lock().map_err(|_| {
        AppError::validation("Hay un backup o una restauración en curso. Espera a que termine.")
    })
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub async fn get_settings(pool: &PgPool) -> Result<BackupSettings, AppError> {
    repository::get_settings(pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::internal("Ajustes de backup no inicializados. Revisa la migración 028.")
        })
}

fn effective_dir(settings: &BackupSettings, default_dir: &Path) -> PathBuf {
    settings
        .local_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dir.to_path_buf())
}

pub async fn update_settings(
    pool: &PgPool,
    mut dto: UpdateBackupSettingsDto,
) -> Result<BackupSettings, AppError> {
    dto.local_dir = clean(dto.local_dir);
    dto.pg_bin_dir = clean(dto.pg_bin_dir);

    if let Some(dir) = &dto.local_dir {
        let path = Path::new(dir);
        if !path.is_absolute() {
            return Err(AppError::validation(
                "La carpeta de backups debe ser una ruta completa (ej: D:\\Backups)",
            ));
        }
        tokio::fs::create_dir_all(path).await.map_err(|e| {
            AppError::Validation(format!("No se pudo usar la carpeta de backups: {e}"))
        })?;
    }

    if let Some(dir) = &dto.pg_bin_dir {
        let exe = Path::new(dir).join(format!("pg_dump{}", pg_tools::EXE_SUFFIX));
        if !exe.is_file() {
            return Err(AppError::validation(
                "La carpeta de PostgreSQL no contiene pg_dump (usa la carpeta bin)",
            ));
        }
    }

    if let Some(n) = dto.keep_daily {
        if !(1..=90).contains(&n) {
            return Err(AppError::validation(
                "Los backups diarios deben estar entre 1 y 90",
            ));
        }
    }
    if let Some(n) = dto.keep_weekly {
        if !(0..=52).contains(&n) {
            return Err(AppError::validation(
                "Los backups semanales deben estar entre 0 y 52",
            ));
        }
    }
    if let Some(n) = dto.keep_monthly {
        if !(0..=60).contains(&n) {
            return Err(AppError::validation(
                "Los backups mensuales deben estar entre 0 y 60",
            ));
        }
    }

    dto.smtp_host = dto.smtp_host.map(|h| h.trim().to_string());
    dto.smtp_security = dto.smtp_security.map(|s| s.trim().to_lowercase());
    dto.smtp_username = clean(dto.smtp_username).map(|u| u.to_lowercase());
    dto.smtp_from_name = clean(dto.smtp_from_name);

    let schedule_time = match dto.schedule_time.as_deref().map(str::trim) {
        None => None,
        Some(text) => Some(
            NaiveTime::parse_from_str(text, "%H:%M")
                .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M:%S"))
                .map_err(|_| AppError::validation("La hora debe tener formato HH:MM"))?,
        ),
    };

    if let Some(mask) = dto.schedule_days {
        if !(1..=127).contains(&mask) {
            return Err(AppError::validation("Elige al menos un día de la semana"));
        }
    }

    let current = get_settings(pool).await?;
    let warn = dto.warn_after_days.unwrap_or(current.warn_after_days);
    let critical = dto
        .critical_after_days
        .unwrap_or(current.critical_after_days);
    if !(1..=30).contains(&warn) {
        return Err(AppError::validation(
            "El aviso debe estar entre 1 y 30 días",
        ));
    }
    if critical <= warn || critical > 60 {
        return Err(AppError::validation(
            "El nivel crítico debe ser mayor que el aviso y no pasar de 60 días",
        ));
    }

    if let Some(host) = &dto.smtp_host {
        if host.is_empty() {
            return Err(AppError::validation(
                "El servidor SMTP no puede estar vacío",
            ));
        }
    }
    if let Some(port) = dto.smtp_port {
        if !(1..=65535).contains(&port) {
            return Err(AppError::validation(
                "El puerto SMTP debe estar entre 1 y 65535",
            ));
        }
    }
    if let Some(security) = &dto.smtp_security {
        if security != "tls" && security != "starttls" {
            return Err(AppError::validation(
                "La seguridad SMTP debe ser TLS o STARTTLS",
            ));
        }
    }
    if let Some(name) = &dto.smtp_from_name {
        if name.chars().count() > 100 {
            return Err(AppError::validation(
                "El nombre del remitente es demasiado largo",
            ));
        }
    }
    if let Some(user) = &dto.smtp_username {
        user.parse::<lettre::Address>()
            .map_err(|_| AppError::validation("El correo remitente no es válido"))?;
    }

    if dto.encrypt_enabled.unwrap_or(current.encrypt_enabled)
        && !secrets::exists(Secret::EncryptionPassword).map_err(AppError::Internal)?
    {
        return Err(AppError::validation(
            "Define la contraseña de cifrado antes de activar el cifrado",
        ));
    }
    if dto.smtp_enabled.unwrap_or(current.smtp_enabled) {
        if dto.smtp_username.is_none() {
            return Err(AppError::validation(
                "Indica el correo remitente (la cuenta de Gmail)",
            ));
        }
        if !secrets::exists(Secret::SmtpPassword).map_err(AppError::Internal)? {
            return Err(AppError::validation(
                "Guarda la contraseña de aplicación del correo antes de activar el envío",
            ));
        }
    }

    repository::update_settings(pool, dto, schedule_time)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::internal("Ajustes de backup no inicializados. Revisa la migración 028.")
        })
}

async fn probe_writable(dir: &Path) -> bool {
    if tokio::fs::create_dir_all(dir).await.is_err() {
        return false;
    }
    let probe = dir.join(".write_test");
    let ok = tokio::fs::write(&probe, b"ok").await.is_ok();
    let _ = tokio::fs::remove_file(&probe).await;
    ok
}

pub async fn check_environment(
    pool: &PgPool,
    default_dir: &Path,
) -> Result<BackupEnvironment, AppError> {
    let settings = get_settings(pool).await?;
    let info = repository::db_info(pool).await.map_err(AppError::from)?;
    let dir = effective_dir(&settings, default_dir);
    let dir_writable = probe_writable(&dir).await;

    let (pg_dump_path, pg_dump_version, tools_error) =
        match pg_tools::locate(settings.pg_bin_dir.as_deref(), info.major()).await {
            Ok(tools) => (
                Some(tools.pg_dump.display().to_string()),
                Some(tools.version_text),
                None,
            ),
            Err(e) => (None, None, Some(e)),
        };

    Ok(BackupEnvironment {
        database: info.database,
        server_version: info.server_version,
        database_size_bytes: info.size_bytes,
        default_dir: default_dir.display().to_string(),
        effective_dir: dir.display().to_string(),
        dir_writable,
        pg_dump_path,
        pg_dump_version,
        tools_error,
    })
}

pub async fn list_history(
    pool: &PgPool,
    limit: Option<i64>,
) -> Result<Vec<BackupRecord>, AppError> {
    repository::list_history(pool, limit.unwrap_or(50).clamp(1, 500))
        .await
        .map_err(AppError::from)
}

struct Artifact {
    file_name: String,
    file_path: PathBuf,
    size_bytes: i64,
    sha256: String,
    pg_server_version: String,
    encrypted: bool,
}

/// Ejecuta un backup completo: dump -> verificar -> hash -> publicar -> historial -> rotación.
/// Los fallos operativos se devuelven como `Validation` porque su mensaje
/// se muestra tal cual al usuario (AppError no tiene una variante para esto).
pub async fn run_backup(
    pool: &PgPool,
    default_dir: &Path,
    kind: BackupKind,
    actor: Option<String>,
) -> Result<BackupRecord, AppError> {
    if restart_pending() {
        return Err(AppError::validation(
            "La aplicación se está reiniciando tras una restauración.",
        ));
    }
    let _guard = acquire()?;
    run_backup_inner(pool, default_dir, kind, actor).await
}

/// Backup previo a una importación masiva. Espera hasta 2 minutos si hay otro
/// en curso. Si falla, el llamador debe CANCELAR la importación.
pub async fn run_pre_import_backup(
    pool: &PgPool,
    default_dir: &Path,
    actor: Option<String>,
) -> Result<BackupRecord, AppError> {
    if restart_pending() {
        return Err(AppError::validation(
            "La aplicación se está reiniciando tras una restauración.",
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(120);
    let _guard = loop {
        match BACKUP_LOCK.try_lock() {
            Ok(guard) => break guard,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(_) => {
                return Err(AppError::validation(
                    "Hay otro backup en curso que tarda demasiado; la importación se canceló. \
                     Inténtalo de nuevo en unos minutos.",
                ));
            }
        }
    };
    run_backup_inner(pool, default_dir, BackupKind::PreImport, actor)
        .await
        .map_err(|e| {
            AppError::Validation(format!(
                "No se pudo crear el backup previo a la importación; la importación se canceló: {e}"
            ))
        })
}

/// Cuerpo del backup. El llamador ya tiene el bloqueo (`acquire`).
pub(crate) async fn run_backup_inner(
    pool: &PgPool,
    default_dir: &Path,
    kind: BackupKind,
    actor: Option<String>,
) -> Result<BackupRecord, AppError> {
    let audit_actor = clean(actor.clone());
    let created_by = clean(actor).unwrap_or_else(|| "system".to_string());
    let settings = get_settings(pool).await?;
    let dir = effective_dir(&settings, default_dir);
    let started_at = Utc::now();
    let timer = Instant::now();

    match execute(pool, &settings, &dir, kind).await {
        Ok(artifact) => {
            let record = repository::insert_success(
                pool,
                SuccessData {
                    kind,
                    started_at,
                    duration_ms: timer.elapsed().as_millis() as i64,
                    file_name: artifact.file_name,
                    file_path: artifact.file_path.display().to_string(),
                    size_bytes: artifact.size_bytes,
                    sha256: artifact.sha256,
                    pg_server_version: artifact.pg_server_version,
                    last_migration: repository::last_migration(pool).await.unwrap_or(0),
                    created_by,
                    encrypted: artifact.encrypted,
                },
            )
            .await
            .map_err(AppError::from)?;

            events::success(pool, kind, &record, audit_actor).await;

            let notify_pool = pool.clone();
            let notify_record = record.clone();
            tokio::spawn(async move {
                notify::after_success(&notify_pool, &notify_record).await;
            });

            tracing::info!(
                file = record.file_name.as_deref().unwrap_or(""),
                size = record.size_bytes.unwrap_or(0),
                ms = record.duration_ms,
                "Backup completado"
            );

            match retention::apply(pool, &settings, &dir).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(removed = n, "Rotación de backups aplicada"),
                Err(e) => tracing::warn!(error = %e, "Falló la rotación de backups"),
            }
            if let Err(e) = retention::apply_pre(pool, &dir).await {
                tracing::warn!(error = %e, "Falló la rotación de backups previos");
            }
            Ok(record)
        }
        Err(message) => {
            tracing::error!(error = %message, "Backup fallido");
            match repository::insert_failure(
                pool,
                kind,
                started_at,
                timer.elapsed().as_millis() as i64,
                &message,
                &created_by,
            )
            .await
            {
                Ok(record) => {
                    let notify_pool = pool.clone();
                    tokio::spawn(async move {
                        notify::after_failure(&notify_pool, &record).await;
                    });
                }
                Err(e) => tracing::warn!(error = %e, "No se pudo registrar el backup fallido"),
            }
            events::failure(pool, kind, &message, audit_actor).await;
            Err(AppError::Validation(message))
        }
    }
}

async fn execute(
    pool: &PgPool,
    settings: &BackupSettings,
    dir: &Path,
    kind: BackupKind,
) -> Result<Artifact, String> {
    tokio::fs::create_dir_all(dir).await.map_err(|e| {
        format!(
            "No se pudo crear la carpeta de backups ({}): {e}",
            dir.display()
        )
    })?;
    let temp_dir = temp::ensure_directory().await?;
    temp::remove_stale(&temp_dir, &["cashflow_backup_"]).await?;
    temp::remove_stale(dir, &["cashflow_backup_"]).await?;
    ensure_readme(dir).await;

    // Si el cifrado está activo y falta la contraseña, el backup FALLA:
    // nunca se hace en claro "por si acaso".
    let password = if settings.encrypt_enabled {
        Some(secrets::get(Secret::EncryptionPassword)?.ok_or_else(|| {
            "El cifrado está activado pero falta la contraseña. Defínela en Ajustes → Backup."
                .to_string()
        })?)
    } else {
        None
    };

    let info = repository::db_info(pool)
        .await
        .map_err(|e| format!("No se pudo consultar la base de datos: {e}"))?;
    let tools = pg_tools::locate(settings.pg_bin_dir.as_deref(), info.major()).await?;
    let conn = ConnParams::from_env()?;

    let stamp = Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let file_name = if password.is_some() {
        format!("cashflow_{stamp}.dump.enc")
    } else {
        format!("cashflow_{stamp}.dump")
    };
    let final_path = dir.join(&file_name);
    let temp_id = uuid::Uuid::new_v4().simple().to_string();
    let out_tmp = dir.join(format!("cashflow_backup_{temp_id}.dump.tmp"));
    // El dump en claro, cuando se cifra, nunca toca la carpeta de backups
    // (puede estar sincronizada con OneDrive).
    let plain_tmp = if password.is_some() {
        temp_dir.join(format!("cashflow_backup_{temp_id}.plain.tmp"))
    } else {
        out_tmp.clone()
    };

    let (size, sha256) = match produce(
        &tools,
        &conn,
        &plain_tmp,
        &out_tmp,
        &final_path,
        password.as_deref(),
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            let _ = tokio::fs::remove_file(&plain_tmp).await;
            let _ = tokio::fs::remove_file(&out_tmp).await;
            return Err(e);
        }
    };

    let meta = BackupMetadata {
        format_version: 1,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now(),
        kind: kind.as_str().to_string(),
        database: info.database.clone(),
        pg_server_version: info.server_version.clone(),
        pg_dump_version: tools.version_text.clone(),
        last_migration: repository::last_migration(pool).await.unwrap_or(0),
        file_name: file_name.clone(),
        size_bytes: size,
        sha256: sha256.clone(),
        encrypted: password.is_some(),
    };
    if let Err(e) = metadata::write(&final_path, &meta).await {
        tracing::warn!(error = %e, "Backup creado, pero sin archivo de metadatos");
    }

    Ok(Artifact {
        file_name,
        file_path: final_path,
        size_bytes: size as i64,
        sha256,
        pg_server_version: info.server_version,
        encrypted: password.is_some(),
    })
}

/// dump -> verificar -> (cifrar -> verificar el cifrado -> borrar el claro) -> hash -> renombrar.
/// El hash y el tamaño son los del archivo que queda en disco (y que se envía).
async fn produce(
    tools: &PgTools,
    conn: &ConnParams,
    plain_tmp: &Path,
    out_tmp: &Path,
    final_path: &Path,
    password: Option<&str>,
) -> Result<(u64, String), String> {
    pg_tools::dump(tools, conn, plain_tmp, DUMP_TIMEOUT).await?;
    pg_tools::verify(tools, plain_tmp, VERIFY_TIMEOUT).await?;

    if let Some(password) = password {
        crypto::encrypt_file(
            plain_tmp.to_path_buf(),
            out_tmp.to_path_buf(),
            password.to_string(),
        )
        .await?;
        crypto::verify_file(out_tmp.to_path_buf(), password.to_string()).await?;
        let _ = tokio::fs::remove_file(plain_tmp).await;
    }

    let size = tokio::fs::metadata(out_tmp)
        .await
        .map_err(|e| format!("No se pudo leer el backup generado: {e}"))?
        .len();
    if size == 0 {
        return Err("El backup generado está vacío".to_string());
    }

    let sha256 = metadata::sha256_file(out_tmp.to_path_buf()).await?;
    tokio::fs::rename(out_tmp, final_path)
        .await
        .map_err(|e| format!("No se pudo finalizar el archivo de backup: {e}"))?;
    Ok((size, sha256))
}

/// Restos de ejecuciones interrumpidas. Es seguro porque el mutex garantiza
/// que no hay otro backup escribiendo ahora.
async fn ensure_readme(dir: &Path) {
    use tokio::io::AsyncWriteExt;
    let path = dir.join("LEEME.txt");
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
    {
        let _ = file.write_all(README_TEXT.as_bytes()).await;
    }
}

fn secrets_state() -> Result<SecretsStatus, AppError> {
    Ok(SecretsStatus {
        encryption_password_set: secrets::exists(Secret::EncryptionPassword)
            .map_err(AppError::Internal)?,
        smtp_password_set: secrets::exists(Secret::SmtpPassword).map_err(AppError::Internal)?,
    })
}

pub fn secrets_status() -> Result<SecretsStatus, AppError> {
    secrets_state()
}

pub async fn set_encryption_password(
    pool: &PgPool,
    password: Option<String>,
) -> Result<SecretsStatus, AppError> {
    match password {
        Some(password) => {
            let length = password.chars().count();
            if !(8..=128).contains(&length) {
                return Err(AppError::validation(
                    "La contraseña de cifrado debe tener entre 8 y 128 caracteres",
                ));
            }
            secrets::set(Secret::EncryptionPassword, &password).map_err(AppError::Internal)?;
            events::config_change(
                pool,
                "Contraseña de cifrado de backups definida".to_string(),
                None,
            )
            .await;
        }
        None => {
            if get_settings(pool).await?.encrypt_enabled {
                return Err(AppError::validation(
                    "Desactiva el cifrado y guarda antes de quitar la contraseña",
                ));
            }
            secrets::delete(Secret::EncryptionPassword).map_err(AppError::Internal)?;
            events::config_change(
                pool,
                "Contraseña de cifrado de backups eliminada".to_string(),
                None,
            )
            .await;
        }
    }
    secrets_state()
}

pub async fn set_smtp_password(
    pool: &PgPool,
    password: Option<String>,
) -> Result<SecretsStatus, AppError> {
    match password {
        Some(password) => {
            // Google muestra la contraseña de aplicación con espacios.
            let cleaned: String = password.chars().filter(|c| !c.is_whitespace()).collect();
            if cleaned.is_empty() || cleaned.chars().count() > 128 {
                return Err(AppError::validation(
                    "La contraseña de aplicación no es válida",
                ));
            }
            secrets::set(Secret::SmtpPassword, &cleaned).map_err(AppError::Internal)?;
            events::config_change(
                pool,
                "Contraseña de correo de backups definida".to_string(),
                None,
            )
            .await;
        }
        None => {
            if get_settings(pool).await?.smtp_enabled {
                return Err(AppError::validation(
                    "Desactiva el envío por correo y guarda antes de quitar la contraseña",
                ));
            }
            secrets::delete(Secret::SmtpPassword).map_err(AppError::Internal)?;
            events::config_change(
                pool,
                "Contraseña de correo de backups eliminada".to_string(),
                None,
            )
            .await;
        }
    }
    secrets_state()
}

fn normalize_email(raw: &str) -> Result<String, AppError> {
    let email = raw.trim().to_lowercase();
    if email.is_empty() || email.len() > 254 {
        return Err(AppError::validation("Escribe un correo válido"));
    }
    email
        .parse::<lettre::Address>()
        .map_err(|_| AppError::validation("El correo no es válido"))?;
    Ok(email)
}

fn clean_name(value: Option<String>) -> Result<Option<String>, AppError> {
    let name = clean(value);
    if name.as_ref().is_some_and(|n| n.chars().count() > 100) {
        return Err(AppError::validation(
            "El nombre es demasiado largo (máximo 100)",
        ));
    }
    Ok(name)
}

pub async fn list_recipients(pool: &PgPool) -> Result<Vec<BackupRecipient>, AppError> {
    repository::list_recipients(pool)
        .await
        .map_err(AppError::from)
}

pub async fn add_recipient(
    pool: &PgPool,
    dto: CreateRecipientDto,
) -> Result<BackupRecipient, AppError> {
    let email = normalize_email(&dto.email)?;
    let name = clean_name(dto.name)?;
    let recipient = repository::insert_recipient(
        pool,
        &email,
        name.as_deref(),
        dto.send_attachment.unwrap_or(true),
    )
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e {
            if db.is_unique_violation() {
                return AppError::validation("Ese correo ya está en la lista");
            }
        }
        AppError::from(e)
    })?;
    events::config_change(
        pool,
        format!("Destinatario de backups añadido: {email}"),
        None,
    )
    .await;
    Ok(recipient)
}

pub async fn update_recipient(
    pool: &PgPool,
    id: i32,
    dto: UpdateRecipientDto,
) -> Result<BackupRecipient, AppError> {
    let name = clean_name(dto.name)?;
    repository::update_recipient(pool, id, name.as_deref(), dto.active, dto.send_attachment)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Destinatario no encontrado"))
}

pub async fn delete_recipient(pool: &PgPool, id: i32) -> Result<(), AppError> {
    let email = repository::delete_recipient(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Destinatario no encontrado"))?;
    events::config_change(
        pool,
        format!("Destinatario de backups eliminado: {email}"),
        None,
    )
    .await;
    Ok(())
}

pub async fn list_deliveries(
    pool: &PgPool,
    limit: Option<i64>,
) -> Result<Vec<BackupDelivery>, AppError> {
    repository::list_deliveries(pool, limit.unwrap_or(30).clamp(1, 200))
        .await
        .map_err(AppError::from)
}

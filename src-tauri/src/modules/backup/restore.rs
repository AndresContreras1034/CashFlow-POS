//! Restores a backup into a separate database before switching it into place.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Connection, PgConnection, PgPool};
use tauri::AppHandle;
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::backup::{
    crypto,
    dto::{ArchiveDetails, BackupInspection, CountItem, RestoreOutcome},
    events,
    models::BackupKind,
    pg_tools::{self, ArchiveHeader, ConnParams, PgTools},
    repository,
    secrets::{self, Secret},
    service,
};

const RESTORE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const LIST_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const KEEP_OLD_DATABASES: usize = 2;
const MAX_LIVE_NAME_LEN: usize = 26;
const COUNTED: [(&str, &str); 4] = [
    ("Ventas", "sales"),
    ("Productos", "products"),
    ("Clientes", "customers"),
    ("Deudas", "debts"),
];

fn safe_ident(name: &str) -> Result<String, String> {
    if !name.is_empty()
        && name.len() <= 63
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(name.to_string())
    } else {
        Err(format!(
            "El nombre de la base «{name}» no es compatible con la restauración \
             (solo letras, números y guion bajo; máximo 63 caracteres)"
        ))
    }
}

fn safe_live_ident(name: &str) -> Result<String, String> {
    if name.len() > MAX_LIVE_NAME_LEN {
        return Err(format!(
            "El nombre de la base actual es demasiado largo para crear de forma segura las \
             bases temporales de restauración (máximo {MAX_LIVE_NAME_LEN} caracteres)"
        ));
    }
    safe_ident(name)
}

fn options(conn: &ConnParams, database: &str) -> PgConnectOptions {
    let mut opts = PgConnectOptions::new()
        .host(&conn.host)
        .port(conn.port)
        .username(&conn.user)
        .database(database);
    if let Some(password) = &conn.password {
        opts = opts.password(password);
    }
    if let Some(mode) = conn.ssl_mode.as_deref() {
        opts = opts.ssl_mode(match mode {
            "disable" => PgSslMode::Disable,
            "allow" => PgSslMode::Allow,
            "require" => PgSslMode::Require,
            "verify-ca" => PgSslMode::VerifyCa,
            "verify-full" => PgSslMode::VerifyFull,
            _ => PgSslMode::Prefer,
        });
    }
    opts
}

async fn open(conn: &ConnParams, database: &str) -> Result<PgConnection, String> {
    PgConnection::connect_with(&options(conn, database))
        .await
        .map_err(|e| format!("No se pudo conectar a la base «{database}»: {e}"))
}

async fn open_admin(conn: &ConnParams) -> Result<PgConnection, String> {
    match open(conn, "postgres").await {
        Ok(connection) => Ok(connection),
        Err(postgres_error) => open(conn, "template1").await.map_err(|template_error| {
            format!(
                "No se pudo conectar a una base administrativa de PostgreSQL \
                 (postgres: {postgres_error}; template1: {template_error})"
            )
        }),
    }
}

async fn exec(conn: &mut PgConnection, statement: String) -> Result<(), sqlx::Error> {
    sqlx::query(&statement)
        .execute(&mut *conn)
        .await
        .map(|_| ())
}

async fn create_scratch(pool: &PgPool, name: &str) -> Result<(), String> {
    let name = safe_ident(name)?;
    sqlx::query(&format!("CREATE DATABASE \"{name}\""))
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| {
            format!(
                "No se pudo crear la base temporal «{name}» \
                 (se requiere permiso CREATEDB y que no exista ya): {e}"
            )
        })
}

async fn drop_scratch(pool: &PgPool, name: &str) -> Result<(), String> {
    let name = safe_ident(name)?;
    sqlx::query(&format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"))
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("No se pudo eliminar la base temporal «{name}»: {e}"))
}

fn app_last_migration() -> i64 {
    sqlx::migrate!("./migrations")
        .iter()
        .map(|migration| migration.version)
        .max()
        .unwrap_or(0)
}

struct TempFile(Option<PathBuf>);

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            if let Err(error) = std::fs::remove_file(path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(%error, "No se pudo borrar el dump temporal descifrado");
                }
            }
        }
    }
}

struct Prepared {
    path: PathBuf,
    encrypted: bool,
    _temp: TempFile,
}

async fn prepare(src: PathBuf, password: Option<String>) -> Result<Prepared, String> {
    let metadata = tokio::fs::metadata(&src)
        .await
        .map_err(|e| format!("No se pudo abrir el archivo: {e}"))?;
    if !metadata.is_file() {
        return Err("La ruta indicada no es un archivo".to_string());
    }

    if !crypto::is_encrypted(src.clone()).await? {
        return Ok(Prepared {
            path: src,
            encrypted: false,
            _temp: TempFile(None),
        });
    }

    let password = password
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Este backup está cifrado: escribe la contraseña".to_string())?;
    let temp = std::env::temp_dir().join(format!(
        "cashflow_restore_{}.plain.tmp",
        Uuid::new_v4().simple()
    ));
    let guard = TempFile(Some(temp.clone()));
    crypto::decrypt_file(src, temp.clone(), password).await?;
    Ok(Prepared {
        path: temp,
        encrypted: true,
        _temp: guard,
    })
}

fn check_server_major(header: &ArchiveHeader, current_major: u32) -> Result<(), String> {
    if let Some(major) = header.server_major() {
        if major > current_major {
            return Err(format!(
                "Este backup se hizo con PostgreSQL {major} y este equipo tiene la versión \
                 {current_major}. Instala una versión igual o más nueva de PostgreSQL."
            ));
        }
    }
    Ok(())
}

async fn counts(connection: &mut PgConnection) -> Result<Vec<CountItem>, String> {
    let mut result = Vec::with_capacity(COUNTED.len());
    for (label, table) in COUNTED {
        let relation: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("public.{table}"))
            .fetch_one(&mut *connection)
            .await
            .map_err(|e| format!("No se pudo comprobar la tabla {table}: {e}"))?;
        if relation.is_none() {
            continue;
        }
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM public.{table}"))
            .fetch_one(&mut *connection)
            .await
            .map_err(|e| format!("No se pudo contar {label}: {e}"))?;
        result.push(CountItem {
            label: label.to_string(),
            count,
        });
    }
    Ok(result)
}

async fn current_counts(pool: &PgPool) -> Result<Vec<CountItem>, String> {
    let mut connection = pool
        .acquire()
        .await
        .map_err(|e| format!("No se pudo consultar la base de datos actual: {e}"))?;
    counts(&mut connection).await
}

async fn read_details(
    connection: &mut PgConnection,
    header: &ArchiveHeader,
) -> Result<ArchiveDetails, String> {
    let last_migration: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0)::bigint FROM _sqlx_migrations")
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| {
                "El archivo no parece un backup de CashFlow POS \
                 (no contiene el historial de migraciones)"
                    .to_string()
            })?;

    let app_last = app_last_migration();
    if last_migration > app_last {
        return Err(format!(
            "Este backup es de una versión más nueva de CashFlow POS (migración {last_migration}); \
             esta instalación llega a la {app_last}. Actualiza la aplicación antes de restaurarlo."
        ));
    }

    let business_name =
        sqlx::query_scalar::<_, String>("SELECT business_name FROM app_settings WHERE id = 1")
            .fetch_optional(&mut *connection)
            .await
            .ok()
            .flatten();

    Ok(ArchiveDetails {
        dumped_server_version: header.server_version.clone(),
        dumped_database: header.database.clone(),
        archive_created_at: header.created_at.clone(),
        last_migration,
        app_last_migration: app_last,
        business_name,
        backup_counts: counts(connection).await?,
        current_counts: Vec::new(),
    })
}

pub async fn inspect(
    pool: &PgPool,
    path: PathBuf,
    password: Option<String>,
) -> Result<BackupInspection, AppError> {
    let _guard = service::acquire()?;
    inspect_inner(pool, &path, password.as_deref())
        .await
        .map_err(AppError::Validation)
}

async fn inspect_inner(
    pool: &PgPool,
    path: &Path,
    password: Option<&str>,
) -> Result<BackupInspection, String> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("No se pudo abrir el archivo: {e}"))?;
    if !metadata.is_file() {
        return Err("La ruta indicada no es un archivo".to_string());
    }
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let encrypted = crypto::is_encrypted(path.to_path_buf()).await?;
    if encrypted && password.map_or(true, str::is_empty) {
        return Ok(BackupInspection {
            file_name,
            size_bytes: metadata.len(),
            encrypted: true,
            needs_password: true,
            details: None,
        });
    }

    let settings = service::get_settings(pool)
        .await
        .map_err(|e| e.to_string())?;
    let info = repository::db_info(pool)
        .await
        .map_err(|e| format!("No se pudo consultar la base de datos: {e}"))?;
    let tools = pg_tools::locate(settings.pg_bin_dir.as_deref(), info.major()).await?;
    let conn = ConnParams::from_env()?;
    let live = safe_live_ident(&conn.database)?;
    let prepared = prepare(path.to_path_buf(), password.map(str::to_string)).await?;
    let listing = pg_tools::list(&tools, &prepared.path, LIST_TIMEOUT).await?;
    let header = pg_tools::parse_header(&listing);
    check_server_major(&header, info.major())?;

    let scratch = format!(
        "{live}_verify_{}",
        &Uuid::new_v4().simple().to_string()[..8]
    );
    create_scratch(pool, &scratch).await?;

    let inspected = async {
        let mut scratch_params = conn.clone();
        scratch_params.database = scratch.clone();
        pg_tools::restore(
            &tools,
            &scratch_params,
            &prepared.path,
            false,
            RESTORE_TIMEOUT,
        )
        .await?;
        let mut connection = open(&conn, &scratch).await?;
        let details = read_details(&mut connection, &header).await;
        connection
            .close()
            .await
            .map_err(|e| format!("No se pudo cerrar la base temporal: {e}"))?;
        details
    }
    .await;

    let removed = drop_scratch(pool, &scratch).await;
    let mut details = match (inspected, removed) {
        (Ok(details), Ok(())) => details,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup_error)) => {
            return Err(format!("{error}. Además, {cleanup_error}"));
        }
    };
    details.current_counts = current_counts(pool).await?;

    Ok(BackupInspection {
        file_name,
        size_bytes: metadata.len(),
        encrypted: prepared.encrypted,
        needs_password: false,
        details: Some(details),
    })
}

struct Adjusted {
    encrypt_enabled: bool,
    smtp_enabled: bool,
    different_machine: bool,
    warnings: Vec<String>,
}

async fn carry_over_license(pool: &PgPool, db: &mut PgConnection) -> Result<bool, String> {
    let live: Option<(String, Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT installation_id, license_file, last_seen_at FROM license_state WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("No se pudo leer la licencia de este equipo: {error}"))?;

    let restored_id: Option<String> = sqlx::query_scalar(
        "SELECT installation_id FROM license_state WHERE id = 1",
    )
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| format!("No se pudo leer la licencia del backup: {error}"))?;

    match live {
        Some((installation_id, license_file, last_seen_at)) => {
            sqlx::query(
                "INSERT INTO license_state (id, installation_id, license_file, last_seen_at)
                 VALUES (1, $1, $2, $3)
                 ON CONFLICT (id) DO UPDATE
                 SET installation_id = EXCLUDED.installation_id,
                     license_file    = EXCLUDED.license_file,
                     last_seen_at    = EXCLUDED.last_seen_at",
            )
            .bind(&installation_id)
            .bind(&license_file)
            .bind(last_seen_at)
            .execute(&mut *db)
            .await
            .map_err(|error| format!("No se pudo conservar la licencia de este equipo: {error}"))?;
            Ok(restored_id.is_some_and(|id| id != installation_id))
        }
        None => {
            sqlx::query("DELETE FROM license_state WHERE id = 1")
                .execute(&mut *db)
                .await
                .map_err(|error| format!("No se pudo descartar la licencia del backup: {error}"))?;
            Ok(restored_id.is_some())
        }
    }
}

async fn adjust_restored_settings(connection: &mut PgConnection) -> Result<Adjusted, String> {
    let row: Option<(Option<String>, Option<String>, bool, bool)> = sqlx::query_as(
        "SELECT local_dir, pg_bin_dir, encrypt_enabled, smtp_enabled
         FROM backup_settings WHERE id = 1",
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(|e| format!("No se pudieron leer los ajustes restaurados: {e}"))?;

    let mut adjusted = Adjusted {
        encrypt_enabled: false,
        smtp_enabled: false,
        different_machine: false,
        warnings: Vec::new(),
    };
    let Some((local_dir, pg_bin, encrypt_enabled, smtp_enabled)) = row else {
        return Ok(adjusted);
    };
    adjusted.encrypt_enabled = encrypt_enabled;
    adjusted.smtp_enabled = smtp_enabled;

    if let Some(dir) = local_dir {
        let path = Path::new(&dir);
        let usable = tokio::fs::metadata(path)
            .await
            .is_ok_and(|metadata| metadata.is_dir());
        if !usable {
            sqlx::query("UPDATE backup_settings SET local_dir = NULL WHERE id = 1")
                .execute(&mut *connection)
                .await
                .map_err(|e| format!("No se pudo ajustar la carpeta de backups: {e}"))?;
            adjusted.warnings.push(format!(
                "La carpeta de backups restaurada ({dir}) no existe o no es válida en este equipo; \
                 se usará la carpeta por defecto."
            ));
        }
    }
    if let Some(dir) = pg_bin {
        let path = Path::new(&dir);
        let has_tools = path
            .join(format!("pg_dump{}", pg_tools::EXE_SUFFIX))
            .is_file()
            && path
                .join(format!("pg_restore{}", pg_tools::EXE_SUFFIX))
                .is_file();
        if !has_tools {
            sqlx::query("UPDATE backup_settings SET pg_bin_dir = NULL WHERE id = 1")
                .execute(&mut *connection)
                .await
                .map_err(|e| format!("No se pudo ajustar la carpeta de PostgreSQL: {e}"))?;
            adjusted.warnings.push(
                "La carpeta de PostgreSQL restaurada no es válida en este equipo; se usará la \
                 detección automática."
                    .to_string(),
            );
        }
    }
    Ok(adjusted)
}

async fn swap_databases(
    admin: &mut PgConnection,
    live: &str,
    fresh: &str,
    old: &str,
) -> Result<(), String> {
    exec(
        admin,
        format!("ALTER DATABASE \"{live}\" WITH ALLOW_CONNECTIONS false"),
    )
    .await
    .map_err(|e| format!("No se pudo bloquear la base de datos actual: {e}"))?;

    let mut renamed = false;
    for _ in 0..20 {
        if let Err(error) = sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity
             WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(live)
        .execute(&mut *admin)
        .await
        {
            let _ = exec(
                admin,
                format!("ALTER DATABASE \"{live}\" WITH ALLOW_CONNECTIONS true"),
            )
            .await;
            return Err(format!(
                "No se pudieron cerrar las conexiones a la base de datos actual: {error}"
            ));
        }
        if exec(
            admin,
            format!("ALTER DATABASE \"{live}\" RENAME TO \"{old}\""),
        )
        .await
        .is_ok()
        {
            renamed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    if !renamed {
        let _ = exec(
            admin,
            format!("ALTER DATABASE \"{live}\" WITH ALLOW_CONNECTIONS true"),
        )
        .await;
        return Err("No se pudieron cerrar las conexiones a la base de datos actual".to_string());
    }

    if let Err(error) = exec(
        admin,
        format!("ALTER DATABASE \"{fresh}\" RENAME TO \"{live}\""),
    )
    .await
    {
        let rollback = exec(
            admin,
            format!("ALTER DATABASE \"{old}\" RENAME TO \"{live}\""),
        )
        .await;
        if rollback.is_ok() {
            let _ = exec(
                admin,
                format!("ALTER DATABASE \"{live}\" WITH ALLOW_CONNECTIONS true"),
            )
            .await;
            return Err(format!(
                "No se pudo activar la base restaurada (se dejó todo como estaba): {error}"
            ));
        }
        return Err(format!(
            "No se pudo activar la base restaurada ({error}) y tampoco se pudo devolver \
             «{old}» a su nombre original. La base anterior permanece guardada como «{old}»."
        ));
    }

    let _ = exec(
        admin,
        format!("ALTER DATABASE \"{old}\" WITH ALLOW_CONNECTIONS true"),
    )
    .await;
    Ok(())
}

async fn cleanup_old(admin: &mut PgConnection, live: &str) -> Result<(), String> {
    let prefix = format!("{live}_pre_restore_");
    let names: Vec<String> =
        sqlx::query_scalar("SELECT datname::text FROM pg_database ORDER BY datname DESC")
            .fetch_all(&mut *admin)
            .await
            .map_err(|e| format!("No se pudieron consultar las restauraciones anteriores: {e}"))?;
    for name in names
        .into_iter()
        .filter(|name| name.starts_with(&prefix))
        .skip(KEEP_OLD_DATABASES)
    {
        let name = safe_ident(&name)?;
        exec(
            admin,
            format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"),
        )
        .await
        .map_err(|e| format!("No se pudo eliminar la base antigua «{name}»: {e}"))?;
    }
    Ok(())
}

struct Swapped {
    file_name: String,
    pre_restore_file: Option<String>,
    last_migration: i64,
    source_encrypted: bool,
    adjusted: Adjusted,
}

#[allow(clippy::too_many_arguments)]
async fn build_and_swap(
    pool: PgPool,
    default_dir: PathBuf,
    tools: PgTools,
    conn: ConnParams,
    prepared: Prepared,
    header: ArchiveHeader,
    live: String,
    fresh: String,
    source_name: String,
    actor: Option<String>,
) -> Result<Swapped, String> {
    let mut fresh_params = conn.clone();
    fresh_params.database = fresh.clone();
    pg_tools::restore(
        &tools,
        &fresh_params,
        &prepared.path,
        false,
        RESTORE_TIMEOUT,
    )
    .await?;
    let mut connection = open(&conn, &fresh).await?;
    let details = read_details(&mut connection, &header).await?;

    let pre_restore =
        service::run_backup_inner(&pool, &default_dir, BackupKind::PreRestore, actor.clone())
            .await
            .map_err(|e| {
                format!(
            "No se pudo crear el backup de seguridad previo; la restauración se canceló: {e}"
        )
            })?;

    connection
        .close()
        .await
        .map_err(|e| format!("No se pudo cerrar la base temporal antes de migrar: {e}"))?;
    let migration_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options(&conn, &fresh))
        .await
        .map_err(|e| format!("No se pudo abrir la base temporal para migrarla: {e}"))?;
    sqlx::migrate!("./migrations")
        .run(&migration_pool)
        .await
        .map_err(|e| {
            format!("No se pudo actualizar la base restaurada a la versión actual: {e}")
        })?;
    migration_pool.close().await;

    let mut connection = open(&conn, &fresh).await?;
    let different_machine = carry_over_license(&pool, &mut connection).await?;
    let mut adjusted = adjust_restored_settings(&mut connection).await?;
    adjusted.different_machine = different_machine;

    if different_machine {
        sqlx::query("UPDATE app_settings SET printer_name = NULL WHERE id = 1")
            .execute(&mut connection)
            .await
            .map_err(|error| format!("No se pudo ajustar la impresora: {error}"))?;
        adjusted.warnings.push(
            "Este backup viene de otro equipo. Se conservó la licencia de ESTE equipo (si aún no \
             está activada, impórtala en la pantalla de licencia) y se quitó la impresora guardada: \
             elige la de este PC en Ajustes."
                .to_string(),
        );
    }

    if let Err(error) = audit_repo::insert_event(
        &mut connection,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Backup,
            action: "restore".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("backup".to_string()),
            entity_id: None,
            summary: format!("Base de datos restaurada desde {source_name}"),
            changes: None,
            metadata: Some(json!({
                "source_file": source_name,
                "source_encrypted": prepared.encrypted,
                "backup_created_at": header.created_at,
                "backup_last_migration": details.last_migration,
                "pre_restore_file": pre_restore.file_path,
                "pre_restore_sha256": pre_restore.sha256,
            })),
            error_message: None,
        },
    )
    .await
    {
        tracing::warn!(%error, "No se pudo registrar la restauración en auditoría");
    }
    connection
        .close()
        .await
        .map_err(|e| format!("No se pudo cerrar la base temporal restaurada: {e}"))?;

    let mut admin = open_admin(&conn).await?;
    let old = format!(
        "{live}_pre_restore_{}_{}",
        Local::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().simple().to_string()[..8]
    );
    let old = safe_ident(&old)?;
    swap_databases(&mut admin, &live, &fresh, &old).await?;
    if let Err(error) = cleanup_old(&mut admin, &live).await {
        tracing::warn!(%error, "La base se restauró, pero no se completó la limpieza de respaldos antiguos");
    }

    Ok(Swapped {
        file_name: source_name.to_string(),
        pre_restore_file: pre_restore.file_path,
        last_migration: details.last_migration,
        source_encrypted: prepared.encrypted,
        adjusted,
    })
}

pub async fn restore(
    app: AppHandle,
    pool: PgPool,
    default_dir: PathBuf,
    path: PathBuf,
    password: Option<String>,
    actor: Option<String>,
) -> Result<RestoreOutcome, AppError> {
    if service::restart_pending() {
        return Err(AppError::validation(
            "La aplicación se está reiniciando tras una restauración.",
        ));
    }
    let _guard = service::acquire()?;

    match restore_inner(
        pool.clone(),
        default_dir.clone(),
        path,
        password.clone(),
        actor.clone(),
    )
    .await
    {
        Ok(outcome) => {
            service::mark_restart_pending();
            tracing::info!("Restauración completada; reiniciando la aplicación");
            let handle = app;
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                handle.restart();
            });
            Ok(outcome)
        }
        Err(message) => {
            tracing::error!(error = %message, "Restauración fallida");
            events::restore_failure(&pool, &message, actor).await;
            Err(AppError::Validation(message))
        }
    }
}

async fn restore_inner(
    pool: PgPool,
    default_dir: PathBuf,
    path: PathBuf,
    password: Option<String>,
    actor: Option<String>,
) -> Result<RestoreOutcome, String> {
    let settings = service::get_settings(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let info = repository::db_info(&pool)
        .await
        .map_err(|e| format!("No se pudo consultar la base de datos: {e}"))?;
    let tools = pg_tools::locate(settings.pg_bin_dir.as_deref(), info.major()).await?;
    let conn = ConnParams::from_env()?;
    let live = safe_live_ident(&conn.database)?;
    let source_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let prepared = prepare(path, password.clone()).await?;
    let listing = pg_tools::list(&tools, &prepared.path, LIST_TIMEOUT).await?;
    let header = pg_tools::parse_header(&listing);
    check_server_major(&header, info.major())?;

    let fresh = format!("{live}_restoring");
    create_scratch(&pool, &fresh).await?;
    let swapped = match build_and_swap(
        pool.clone(),
        default_dir,
        tools,
        conn,
        prepared,
        header,
        live,
        fresh.clone(),
        source_name,
        actor,
    )
    .await
    {
        Ok(swapped) => swapped,
        Err(error) => {
            return match drop_scratch(&pool, &fresh).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!("{error}. Además, {cleanup_error}")),
            };
        }
    };

    let mut warnings = swapped.adjusted.warnings;
    if swapped.adjusted.encrypt_enabled {
        match secrets::exists(Secret::EncryptionPassword) {
            Ok(false) => match (swapped.source_encrypted, password) {
                (true, Some(password)) => {
                    match secrets::set(Secret::EncryptionPassword, &password) {
                        Ok(()) => warnings.push(
                            "La contraseña de cifrado de este backup se guardó en este equipo \
                             para los próximos backups."
                                .to_string(),
                        ),
                        Err(error) => warnings.push(format!(
                            "No se pudo guardar la contraseña de cifrado en este equipo ({error}). \
                             Defínela en Ajustes → Backup o los backups fallarán."
                        )),
                    }
                }
                _ => warnings.push(
                    "Los backups están configurados como cifrados, pero en este equipo falta la \
                     contraseña de cifrado. Defínela en Ajustes → Backup o los backups fallarán."
                        .to_string(),
                ),
            },
            Ok(true) => {}
            Err(error) => warnings.push(format!(
                "No se pudo consultar la contraseña de cifrado guardada ({error}); verifica \
                 Ajustes → Backup."
            )),
        }
    }
    if swapped.adjusted.smtp_enabled {
        match secrets::exists(Secret::SmtpPassword) {
            Ok(false) => warnings.push(
                "El envío por correo está activo, pero la contraseña de aplicación de Gmail no \
                 viaja en el backup. Vuelve a introducirla en Ajustes → Backup."
                    .to_string(),
            ),
            Ok(true) => {}
            Err(error) => warnings.push(format!(
                "No se pudo consultar la contraseña SMTP guardada ({error}); verifica \
                 Ajustes → Backup."
            )),
        }
    }

    Ok(RestoreOutcome {
        file_name: swapped.file_name,
        pre_restore_file: swapped.pre_restore_file,
        last_migration: swapped.last_migration,
        warnings,
        restarting: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_restricted() {
        assert!(safe_ident("pos_db").is_ok());
        assert!(safe_ident("pos-db").is_err());
        assert!(safe_ident("pos\"; DROP").is_err());
        assert!(safe_ident("").is_err());
        assert!(safe_live_ident(&"x".repeat(MAX_LIVE_NAME_LEN + 1)).is_err());
    }
}

//! Localiza y ejecuta pg_dump / pg_restore.
//! Los errores son `String` ya legibles para el usuario.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

#[cfg(windows)]
pub const EXE_SUFFIX: &str = ".exe";
#[cfg(not(windows))]
pub const EXE_SUFFIX: &str = "";

#[derive(Debug, Clone)]
pub struct PgTools {
    pub pg_dump: PathBuf,
    pub pg_restore: PathBuf,
    pub major: u32,
    pub version_text: String,
}

#[derive(Debug, Clone)]
pub struct ConnParams {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    pub ssl_mode: Option<String>,
}

impl ConnParams {
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL no está definida".to_string())?;
        Self::from_database_url(&raw)
    }

    pub fn from_database_url(raw: &str) -> Result<Self, String> {
        let url = url::Url::parse(raw.trim())
            .map_err(|e| format!("DATABASE_URL no es una URL válida: {e}"))?;
        let decode = |s: &str| {
            percent_encoding::percent_decode_str(s)
                .decode_utf8_lossy()
                .into_owned()
        };

        let host = url
            .host_str()
            .filter(|h| !h.is_empty())
            .unwrap_or("localhost")
            .trim_matches(|c| c == '[' || c == ']')
            .to_string();
        let port = url.port().unwrap_or(5432);
        let user = match decode(url.username()) {
            u if u.is_empty() => "postgres".to_string(),
            u => u,
        };
        let password = url.password().map(decode);
        let database = decode(url.path().trim_start_matches('/'));
        if database.is_empty() {
            return Err("DATABASE_URL no indica el nombre de la base de datos".to_string());
        }
        let ssl_mode = url
            .query_pairs()
            .find(|(k, _)| k == "sslmode")
            .map(|(_, v)| v.into_owned());

        Ok(Self {
            host,
            port,
            user,
            password,
            database,
            ssl_mode,
        })
    }
}

fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS
        cmd.creation_flags(0x0800_0000 | 0x0000_4000);
    }
    cmd
}

fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let count = text.chars().count();
    if count > 1500 {
        text.chars().skip(count - 1500).collect()
    } else {
        text.to_string()
    }
}

/// "pg_dump (PostgreSQL) 18.4" -> 18
pub fn parse_major(text: &str) -> Option<u32> {
    text.split_whitespace()
        .find(|token| token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .and_then(|token| {
            let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().ok()
        })
}

async fn run_version(exe: &Path) -> Result<(u32, String), String> {
    let mut cmd = command(exe);
    cmd.arg("--version");
    let output = tokio::time::timeout(Duration::from_secs(15), cmd.output())
        .await
        .map_err(|_| "tardó demasiado en responder".to_string())?
        .map_err(|e| format!("no se pudo ejecutar: {e}"))?;
    if !output.status.success() {
        return Err(format!("falló: {}", tail(&output.stderr)));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let major = parse_major(&text).ok_or_else(|| format!("versión ilegible: {text}"))?;
    Ok((major, text))
}

async fn probe(
    pg_dump: PathBuf,
    pg_restore: PathBuf,
    server_major: u32,
) -> Result<PgTools, String> {
    let (major, version_text) = run_version(&pg_dump).await?;
    if major < server_major {
        return Err(format!(
            "pg_dump {major} es más antiguo que el servidor PostgreSQL {server_major}"
        ));
    }
    Ok(PgTools {
        pg_dump,
        pg_restore,
        major,
        version_text,
    })
}

async fn probe_dir(dir: &Path, server_major: u32) -> Result<PgTools, String> {
    let pg_dump = dir.join(format!("pg_dump{EXE_SUFFIX}"));
    let pg_restore = dir.join(format!("pg_restore{EXE_SUFFIX}"));
    if !pg_dump.is_file() {
        return Err("no contiene pg_dump".to_string());
    }
    if !pg_restore.is_file() {
        return Err("no contiene pg_restore".to_string());
    }
    probe(pg_dump, pg_restore, server_major).await
}

/// Carpetas `bin` de instalaciones típicas, de la versión más nueva a la más vieja.
fn installed_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(var) {
            roots.push(PathBuf::from(base).join("PostgreSQL"));
        }
    }
    let mut found: Vec<(u32, PathBuf)> = Vec::new();
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                if let Some(version) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                {
                    found.push((version, entry.path().join("bin")));
                }
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, path)| path).collect()
}

/// Orden: carpeta de Ajustes -> variable PG_BIN_DIR -> instalaciones en
/// Program Files -> PATH. Si el usuario fijó una carpeta y no sirve, falla
/// con ese motivo en vez de buscar en otro sitio sin avisar.
pub async fn locate(custom_dir: Option<&str>, server_major: u32) -> Result<PgTools, String> {
    if let Some(dir) = custom_dir.map(str::trim).filter(|d| !d.is_empty()) {
        return probe_dir(Path::new(dir), server_major).await.map_err(|e| {
            format!("La carpeta de PostgreSQL configurada en Ajustes no sirve ({dir}): {e}")
        });
    }

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("PG_BIN_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    dirs.extend(installed_dirs());

    let mut last_error: Option<String> = None;
    for dir in &dirs {
        match probe_dir(dir, server_major).await {
            Ok(tools) => return Ok(tools),
            Err(e) => last_error = Some(format!("{}: {e}", dir.display())),
        }
    }

    match probe(
        PathBuf::from(format!("pg_dump{EXE_SUFFIX}")),
        PathBuf::from(format!("pg_restore{EXE_SUFFIX}")),
        server_major,
    )
    .await
    {
        Ok(tools) => Ok(tools),
        Err(e) => Err(format!(
            "No se encontró un pg_dump compatible con PostgreSQL {server_major}. \
             Indica la carpeta bin de PostgreSQL en Ajustes. Último intento: {}",
            last_error.unwrap_or(e)
        )),
    }
}

/// Genera el dump en formato custom (comprimido, restaurable con pg_restore).
/// La contraseña viaja por PGPASSWORD, nunca en la línea de comandos.
pub async fn dump(
    tools: &PgTools,
    conn: &ConnParams,
    out_file: &Path,
    timeout: Duration,
) -> Result<(), String> {
    let mut cmd = command(&tools.pg_dump);
    cmd.arg("--host")
        .arg(&conn.host)
        .arg("--port")
        .arg(conn.port.to_string())
        .arg("--username")
        .arg(&conn.user)
        .arg("--dbname")
        .arg(&conn.database)
        .arg("--format=custom")
        .arg("--compress=6")
        .arg("--no-password")
        .arg("--lock-wait-timeout=60000")
        .arg("--file")
        .arg(out_file);
    cmd.env("PGCONNECT_TIMEOUT", "15");
    if let Some(password) = &conn.password {
        cmd.env("PGPASSWORD", password);
    }
    if let Some(mode) = &conn.ssl_mode {
        cmd.env("PGSSLMODE", mode);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo iniciar pg_dump: {e}"))?;

    // Si vence el plazo, el future se descarta y kill_on_drop mata el proceso.
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| {
            format!(
                "pg_dump superó el tiempo máximo ({} min) y se canceló",
                timeout.as_secs() / 60
            )
        })?
        .map_err(|e| format!("pg_dump falló al ejecutarse: {e}"))?;

    if !output.status.success() {
        return Err(format!("pg_dump terminó con error: {}", tail(&output.stderr)));
    }
    Ok(())
}

/// `pg_restore --list` lee todo el índice del archivo: si está corrupto o no
/// es un dump válido, falla aquí.
pub async fn verify(tools: &PgTools, file: &Path, timeout: Duration) -> Result<(), String> {
    let mut cmd = command(&tools.pg_restore);
    cmd.arg("--list").arg(file);
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo iniciar pg_restore: {e}"))?;
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| "La verificación del backup tardó demasiado".to_string())?
        .map_err(|e| format!("pg_restore falló al ejecutarse: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "El backup generado no pasó la verificación: {}",
            tail(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_major_version() {
        assert_eq!(parse_major("pg_dump (PostgreSQL) 18.4"), Some(18));
        assert_eq!(parse_major("pg_dump (PostgreSQL) 9.6.24"), Some(9));
        assert_eq!(parse_major("sin versión"), None);
    }

    #[test]
    fn parses_database_url() {
        let c = ConnParams::from_database_url(
            "postgres://postgres:p%40ss%3A1@localhost:5433/pos_db?sslmode=disable",
        )
        .unwrap();
        assert_eq!(c.host, "localhost");
        assert_eq!(c.port, 5433);
        assert_eq!(c.user, "postgres");
        assert_eq!(c.password.as_deref(), Some("p@ss:1"));
        assert_eq!(c.database, "pos_db");
        assert_eq!(c.ssl_mode.as_deref(), Some("disable"));
    }

    #[test]
    fn database_url_requires_database_name() {
        assert!(ConnParams::from_database_url("postgres://u:p@localhost:5432").is_err());
    }
}

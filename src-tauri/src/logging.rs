//! Formato de logs para desarrollo.

use std::fmt::{self, Write as _};
use std::io::{IsTerminal, Write as _};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

use crate::db::connection::DbHealth;
use crate::modules::developer::{self, state::DeveloperModeState};

pub const DEFAULT_FILTER: &str = "warn,pos_lib=info,sqlx=warn";

const TARGET_WIDTH: usize = 22;
const LABEL_WIDTH: usize = 7;
const BANNER_WIDTH: usize = 54;
const MAX_DETAIL_CHARS: usize = 300;
const ELLIPSIS: &str = "...";
const DETAIL_PREFIX: &str = "+- ";
const DIM: &str = "90";

/// Un WARN/ERROR idéntico al anterior, dentro de esta ventana, no se vuelve a
/// imprimir: se cuenta y se resume al terminar la racha.
const REPEAT_WINDOW: Duration = Duration::from_secs(5);
const FLUSH_TICK: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Ok,
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

impl Tag {
    pub fn label(self) -> &'static str {
        match self {
            Tag::Ok => "[ OK  ]",
            Tag::Info => "[INFO ]",
            Tag::Warn => "[WARN ]",
            Tag::Error => "[ERROR]",
            Tag::Debug => "[DEBUG]",
            Tag::Trace => "[TRACE]",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Tag::Ok => "32",
            Tag::Info => "36",
            Tag::Warn => "33",
            Tag::Error => "31",
            Tag::Debug | Tag::Trace => DIM,
        }
    }
}

pub fn tag_for(level: &Level, ok: bool) -> Tag {
    match *level {
        Level::ERROR => Tag::Error,
        Level::WARN => Tag::Warn,
        Level::INFO => {
            if ok {
                Tag::Ok
            } else {
                Tag::Info
            }
        }
        Level::DEBUG => Tag::Debug,
        Level::TRACE => Tag::Trace,
    }
}

/// `pos_lib::modules::audit::failure` -> `audit::failure`.
pub fn short_target(target: &str, verbose: bool) -> String {
    if verbose {
        return target.to_string();
    }
    if target == "pos_lib" {
        return "app".to_string();
    }
    match target.strip_prefix("pos_lib::") {
        Some(inner) => inner.strip_prefix("modules::").unwrap_or(inner).to_string(),
        None => target.to_string(),
    }
}

/// Ajusta el target a un ancho fijo para que la columna `|` no se mueva.
/// Si no cabe, conserva el final (lo más específico) con un `~` delante.
pub fn fit_target(target: &str, width: usize) -> String {
    let len = target.chars().count();
    if len <= width {
        return format!("{:<width$}", target, width = width);
    }
    let tail: String = target.chars().skip(len - (width - 1)).collect();
    format!("~{tail}")
}

pub fn colors_enabled(is_terminal: bool, no_color_set: bool) -> bool {
    is_terminal && !no_color_set
}

pub fn is_verbose(max_level_hint: Option<LevelFilter>) -> bool {
    max_level_hint.is_some_and(|level| level >= LevelFilter::DEBUG)
}

pub fn detail_label(name: &str) -> &str {
    match name {
        "error" => "motivo",
        other => other,
    }
}

pub fn redact_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("://") {
        let (head, tail) = rest.split_at(pos + 3);
        out.push_str(head);
        let token_end = tail.find(char::is_whitespace).unwrap_or(tail.len());
        match tail[..token_end].rfind('@') {
            Some(at) => {
                out.push_str("***");
                rest = &tail[at..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

/// Aplana, redacta y limita a `MAX_DETAIL_CHARS` caracteres EN TOTAL
/// (los `...` cuentan). Primero redacta, después trunca.
pub fn clean_value(value: &str) -> String {
    let flat = redact_credentials(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if flat.chars().count() > MAX_DETAIL_CHARS {
        let keep = MAX_DETAIL_CHARS - ELLIPSIS.chars().count();
        let mut cut: String = flat.chars().take(keep).collect();
        cut.push_str(ELLIPSIS);
        cut
    } else {
        flat
    }
}

fn paint(color: bool, code: &str, text: &str) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn render_event(
    time: &str,
    tag: Tag,
    target: &str,
    message: &str,
    details: &[(String, String)],
    color: bool,
) -> String {
    let padded_target = fit_target(target, TARGET_WIDTH);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} {} {} | {}",
        paint(color, DIM, time),
        paint(color, tag.color(), tag.label()),
        paint(color, DIM, &padded_target),
        message
    );

    let indent = " ".repeat(time.len() + 1 + LABEL_WIDTH + 1);
    for (name, value) in details {
        let _ = writeln!(
            out,
            "{indent}{DETAIL_PREFIX}{}: {}",
            detail_label(name),
            clean_value(value)
        );
    }
    out
}

// ============================================================
// RACHAS DE WARN/ERROR REPETIDOS
// ============================================================

/// Lo que se muestra al terminar una racha: `^ x6 en total durante 2.1 s`.
#[derive(Debug, Clone, PartialEq)]
pub struct RepeatSummary {
    pub tag: Tag,
    pub target: String,
    pub total: u32,
    pub last_time: String,
    pub span: Duration,
}

pub fn render_repeat(summary: &RepeatSummary, color: bool) -> String {
    format!(
        "{} {} {} | ^ x{} en total durante {:.1} s\n",
        paint(color, DIM, &summary.last_time),
        paint(color, summary.tag.color(), summary.tag.label()),
        paint(color, DIM, &fit_target(&summary.target, TARGET_WIDTH)),
        summary.total,
        summary.span.as_secs_f32()
    )
}

/// Dos eventos son "el mismo" si coinciden nivel, target, mensaje y detalles.
pub fn repeat_key(tag: Tag, target: &str, message: &str, details: &[(String, String)]) -> String {
    let details = details
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(";");
    format!("{}|{}|{}|{}", tag.label(), target, message, details)
}

pub struct Candidate<'a> {
    pub key: &'a str,
    pub collapsible: bool,
    pub tag: Tag,
    pub target: &'a str,
    pub time: &'a str,
}

pub struct Observed {
    /// No imprimir este evento: repite al anterior.
    pub suppress: bool,
    /// Resumen de la racha anterior, que debe imprimirse antes.
    pub flush: Option<RepeatSummary>,
}

struct Streak {
    key: String,
    tag: Tag,
    target: String,
    first_at: Instant,
    last_at: Instant,
    last_time: String,
    suppressed: u32,
}

impl Streak {
    fn summary(self) -> Option<RepeatSummary> {
        (self.suppressed > 0).then(|| RepeatSummary {
            tag: self.tag,
            target: self.target,
            total: self.suppressed + 1,
            last_time: self.last_time,
            span: self.last_at.duration_since(self.first_at),
        })
    }
}

/// Máquina de estados pura (el reloj entra como parámetro, para probarla).
#[derive(Default)]
pub struct RepeatGuard {
    streak: Option<Streak>,
}

impl RepeatGuard {
    pub fn observe(&mut self, event: &Candidate<'_>, now: Instant) -> Observed {
        if event.collapsible {
            if let Some(streak) = self.streak.as_mut() {
                if streak.key == event.key && now.duration_since(streak.last_at) < REPEAT_WINDOW {
                    streak.suppressed += 1;
                    streak.last_at = now;
                    streak.last_time = event.time.to_string();
                    return Observed {
                        suppress: true,
                        flush: None,
                    };
                }
            }
        }

        // No es repetición: cualquier evento distinto cierra la racha anterior.
        let flush = self.streak.take().and_then(Streak::summary);
        if event.collapsible {
            self.streak = Some(Streak {
                key: event.key.to_string(),
                tag: event.tag,
                target: event.target.to_string(),
                first_at: now,
                last_at: now,
                last_time: event.time.to_string(),
                suppressed: 0,
            });
        }
        Observed {
            suppress: false,
            flush,
        }
    }

    /// Si la racha lleva `REPEAT_WINDOW` sin repetirse, la cierra y la resume.
    pub fn take_expired(&mut self, now: Instant) -> Option<RepeatSummary> {
        let expired = self
            .streak
            .as_ref()
            .is_some_and(|streak| now.duration_since(streak.last_at) >= REPEAT_WINDOW);
        if expired {
            self.streak.take().and_then(Streak::summary)
        } else {
            None
        }
    }
}

fn lock_guard(guard: &Mutex<RepeatGuard>) -> MutexGuard<'_, RepeatGuard> {
    guard
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Sin este hilo, una racha que termina sin que llegue otro log nunca
/// mostraría su contador.
fn spawn_repeat_flusher(guard: Arc<Mutex<RepeatGuard>>, color: bool) {
    let _ = std::thread::Builder::new()
        .name("log-repeat-flush".into())
        .spawn(move || loop {
            std::thread::sleep(FLUSH_TICK);
            let summary = lock_guard(&guard).take_expired(Instant::now());
            if let Some(summary) = summary {
                let _ = std::io::stdout().write_all(render_repeat(&summary, color).as_bytes());
            }
        });
}

pub fn banner() -> String {
    let title = "CASHFLOW POS - DEV MODE";
    let border = format!("+{}+", "-".repeat(BANNER_WIDTH));
    let pad_total = BANNER_WIDTH - title.len();
    let left = pad_total / 2;
    let right = pad_total - left;
    format!(
        "{border}\n|{}{title}{}|\n{border}\n",
        " ".repeat(left),
        " ".repeat(right)
    )
}

#[derive(Default)]
pub(crate) struct EventFields {
    pub(crate) message: String,
    pub(crate) ok: bool,
    pub(crate) details: Vec<(String, String)>,
}

impl EventFields {
    fn push(&mut self, field: &Field, value: String) {
        if field.name().starts_with("log.") {
            return;
        }
        self.details.push((field.name().to_string(), value));
    }
}

impl Visit for EventFields {
    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "ok" {
            self.ok = value;
        } else {
            self.push(field, value.to_string());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            self.push(field, value.to_string());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            self.push(field, format!("{value:?}"));
        }
    }
}

pub struct DevFormat {
    color: bool,
    verbose: bool,
    repeats: Arc<Mutex<RepeatGuard>>,
}

impl<S, N> FormatEvent<S, N> for DevFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = EventFields::default();
        event.record(&mut fields);

        let meta = event.metadata();
        let time_format = if self.verbose {
            "%H:%M:%S%.3f"
        } else {
            "%H:%M:%S"
        };
        let time = chrono::Local::now().format(time_format).to_string();
        let tag = tag_for(meta.level(), fields.ok);
        let target = short_target(meta.target(), self.verbose);
        let message = redact_credentials(&fields.message);

        let key = repeat_key(tag, &target, &message, &fields.details);
        let observed = lock_guard(&self.repeats).observe(
            &Candidate {
                key: &key,
                collapsible: matches!(tag, Tag::Warn | Tag::Error),
                tag,
                target: &target,
                time: &time,
            },
            Instant::now(),
        );

        if let Some(summary) = &observed.flush {
            writer.write_str(&render_repeat(summary, self.color))?;
        }
        if observed.suppress {
            return Ok(());
        }

        let line = render_event(&time, tag, &target, &message, &fields.details, self.color);
        writer.write_str(&line)
    }
}

// ============================================================
// PANEL DE ARRANQUE
// ============================================================

const PANEL_LABEL_WIDTH: usize = 14;
const PANEL_TAG_SLOT: usize = LABEL_WIDTH + 1;

/// Todo lo que el panel muestra, ya calculado. `render_startup_panel`
/// no consulta nada por su cuenta.
pub struct StartupInfo {
    pub app_version: &'static str,
    pub profile: &'static str,
    pub log_filter: String,
    /// `Err` lleva solo el texto del error (se redacta al imprimir).
    pub db: Result<DbHealth, String>,
    pub pool_size: u32,
    pub pool_idle: usize,
    pub pool_max: u32,
    pub startup: Duration,
}

/// "PostgreSQL 18.0 on x86_64-pc-windows, compiled by..." -> "PostgreSQL 18.0".
pub fn short_pg_version(full: &str) -> String {
    let short = full
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let short = short.trim_end_matches(',');
    if short.is_empty() {
        "version desconocida".to_string()
    } else {
        short.to_string()
    }
}

/// Filtro que el usuario ve: `RUST_LOG` si tiene contenido, si no el de por defecto.
pub fn effective_filter(rust_log: Option<&str>) -> String {
    match rust_log.map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => DEFAULT_FILTER.to_string(),
    }
}

pub fn format_millis(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

fn panel_row(out: &mut String, label: &str, tag: Option<Tag>, value: &str, color: bool) {
    let slot = match tag {
        Some(tag) => format!("{} ", paint(color, tag.color(), tag.label())),
        None => " ".repeat(PANEL_TAG_SLOT),
    };
    let label = format!("{label:<width$}", width = PANEL_LABEL_WIDTH);
    let _ = writeln!(out, "  {}{}{}", paint(color, DIM, &label), slot, value);
}

pub fn render_startup_panel(info: &StartupInfo, color: bool) -> String {
    let mut out = String::new();

    panel_row(
        &mut out,
        "App",
        None,
        &format!(
            "v{}, {}, log: {}",
            info.app_version,
            info.profile,
            clean_value(&info.log_filter)
        ),
        color,
    );

    match &info.db {
        Ok(db) => panel_row(
            &mut out,
            "Base de datos",
            Some(Tag::Ok),
            &format!(
                "{}, {}, {}",
                clean_value(&db.database),
                short_pg_version(&db.server_version),
                format_millis(db.latency)
            ),
            color,
        ),
        Err(error) => panel_row(
            &mut out,
            "Base de datos",
            Some(Tag::Warn),
            &format!("consulta de salud fallida: {}", clean_value(error)),
            color,
        ),
    }

    panel_row(
        &mut out,
        "Pool",
        None,
        &format!(
            "{} abiertas, {} libres, max {}",
            info.pool_size, info.pool_idle, info.pool_max
        ),
        color,
    );
    panel_row(&mut out, "Migraciones", Some(Tag::Ok), "aplicadas", color);
    panel_row(
        &mut out,
        "Arranque",
        None,
        &format!(
            "{:.1} s hasta base de datos lista",
            info.startup.as_secs_f32()
        ),
        color,
    );
    panel_row(
        &mut out,
        "Developer",
        None,
        "Ajustes > Desarrollador (/developer)",
        color,
    );
    panel_row(
        &mut out,
        "Diagnostico",
        None,
        "RUST_LOG=warn,pos_lib=debug",
        color,
    );

    let _ = writeln!(out, "{}", "-".repeat(BANNER_WIDTH + 2));
    out
}

#[derive(Clone, Copy)]
struct ConsoleMode {
    terminal: bool,
    color: bool,
}

/// Lo decide `init` una vez; el panel reutiliza la misma decision.
static CONSOLE: OnceLock<ConsoleMode> = OnceLock::new();

/// Solo en debug y solo si stdout es una terminal (igual que el banner).
pub fn print_startup_panel(info: &StartupInfo) {
    let Some(mode) = CONSOLE.get() else {
        return;
    };
    if !(cfg!(debug_assertions) && mode.terminal) {
        return;
    }
    let text = format!("\n{}", render_startup_panel(info, mode.color));
    let _ = std::io::stdout().write_all(text.as_bytes());
}

#[cfg(windows)]
fn enable_ansi() -> bool {
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        if mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 {
            return true;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(not(windows))]
fn enable_ansi() -> bool {
    true
}

pub fn init(developer: Arc<DeveloperModeState>) {
    let stdout_is_terminal = std::io::stdout().is_terminal();
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    let color = colors_enabled(stdout_is_terminal, no_color) && enable_ansi();
    let _ = CONSOLE.set(ConsoleMode {
        terminal: stdout_is_terminal,
        color,
    });

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let verbose = is_verbose(<EnvFilter as Layer<Registry>>::max_level_hint(&filter));

    if cfg!(debug_assertions) && stdout_is_terminal {
        println!("{}", banner());
    }

    let repeats = Arc::new(Mutex::new(RepeatGuard::default()));
    spawn_repeat_flusher(repeats.clone(), color);

    // El filtro del entorno aplica solo a la consola: Developer Mode
    // lleva el suyo (WARN/ERROR de pos_lib).
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(DevFormat {
            color,
            verbose,
            repeats,
        })
        .with_filter(filter);

    let _ = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(developer::layer::build(developer))
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_target_strips_crate_and_modules_prefix() {
        assert_eq!(
            short_target("pos_lib::db::connection", false),
            "db::connection"
        );
        assert_eq!(
            short_target("pos_lib::modules::audit::failure", false),
            "audit::failure"
        );
        assert_eq!(short_target("pos_lib", false), "app");
        assert_eq!(short_target("sqlx::query", false), "sqlx::query");
        assert_eq!(
            short_target("other::modules::x", false),
            "other::modules::x"
        );
        assert_eq!(
            short_target("pos_lib::modules::audit::failure", true),
            "pos_lib::modules::audit::failure"
        );
    }

    #[test]
    fn fit_target_pads_and_truncates_to_fixed_width() {
        assert_eq!(fit_target("app", 8), "app     ");
        assert_eq!(fit_target("exactly8", 8).chars().count(), 8);
        let long = fit_target("inventory::repository", 10);
        assert_eq!(long.chars().count(), 10);
        assert!(long.starts_with('~'));
        assert_eq!(long, "~epository");
    }

    #[test]
    fn long_targets_do_not_break_the_separator_column() {
        let a = render_event("09:14:25", Tag::Info, "db::connection", "x", &[], false);
        let b = render_event(
            "09:14:25",
            Tag::Error,
            "inventory::repository::deep",
            "y",
            &[],
            false,
        );
        assert_eq!(a.find('|'), b.find('|'));
    }

    #[test]
    fn ok_only_applies_to_info() {
        assert_eq!(tag_for(&Level::INFO, true), Tag::Ok);
        assert_eq!(tag_for(&Level::INFO, false), Tag::Info);
        assert_eq!(tag_for(&Level::WARN, true), Tag::Warn);
        assert_eq!(tag_for(&Level::ERROR, true), Tag::Error);
        assert_eq!(tag_for(&Level::DEBUG, false), Tag::Debug);
    }

    #[test]
    fn labels_have_fixed_width() {
        for tag in [
            Tag::Ok,
            Tag::Info,
            Tag::Warn,
            Tag::Error,
            Tag::Debug,
            Tag::Trace,
        ] {
            assert_eq!(tag.label().len(), LABEL_WIDTH);
        }
    }

    #[test]
    fn colors_need_terminal_and_no_no_color() {
        assert!(colors_enabled(true, false));
        assert!(!colors_enabled(false, false));
        assert!(!colors_enabled(true, true));
    }

    #[test]
    fn verbose_starts_at_debug() {
        assert!(!is_verbose(None));
        assert!(!is_verbose(Some(LevelFilter::INFO)));
        assert!(!is_verbose(Some(LevelFilter::OFF)));
        assert!(is_verbose(Some(LevelFilter::DEBUG)));
        assert!(is_verbose(Some(LevelFilter::TRACE)));
    }

    #[test]
    fn redacts_credentials_in_urls() {
        assert_eq!(
            redact_credentials("falló postgres://usuario:clave@localhost:5432/pos_db fin"),
            "falló postgres://***@localhost:5432/pos_db fin"
        );
        assert_eq!(
            redact_credentials("postgres://localhost:5432/pos"),
            "postgres://localhost:5432/pos"
        );
        assert_eq!(redact_credentials("sin url"), "sin url");
    }

    #[test]
    fn clean_value_flattens_and_truncates() {
        assert_eq!(clean_value("a\nb   c"), "a b c");
        let long = "x".repeat(400);
        let cut = clean_value(&long);
        assert_eq!(cut.chars().count(), MAX_DETAIL_CHARS);
        assert!(cut.ends_with("..."));
    }

    #[test]
    fn clean_value_keeps_exactly_the_limit_untouched() {
        let exact = "x".repeat(MAX_DETAIL_CHARS);
        assert_eq!(clean_value(&exact), exact);
        let over = clean_value(&"x".repeat(MAX_DETAIL_CHARS + 1));
        assert_eq!(over.chars().count(), MAX_DETAIL_CHARS);
        assert!(over.ends_with("..."));
    }

    #[test]
    fn clean_value_uses_char_count_not_byte_count() {
        assert_eq!(
            clean_value(&"ñ".repeat(400)).chars().count(),
            MAX_DETAIL_CHARS
        );
    }

    #[test]
    fn clean_value_redacts_before_truncating() {
        let text = format!("{} postgres://usuario:clave@host/db", "x".repeat(290));
        assert!(!clean_value(&text).contains("clave"));
    }

    #[test]
    fn error_field_is_shown_as_motivo() {
        assert_eq!(detail_label("error"), "motivo");
        assert_eq!(detail_label("correlation_id"), "correlation_id");
    }

    #[test]
    fn plain_line_has_expected_shape() {
        let line = render_event(
            "09:14:25",
            Tag::Ok,
            "db::connection",
            "Migraciones aplicadas",
            &[],
            false,
        );
        assert!(line.starts_with("09:14:25 [ OK  ] db::connection"));
        assert!(line.contains(" | Migraciones aplicadas"));
        assert!(line.ends_with('\n'));
        assert!(!line.contains('\x1b'));
    }

    #[test]
    fn separator_column_is_aligned() {
        let a = render_event("09:14:25", Tag::Info, "db::connection", "x", &[], false);
        let b = render_event("09:14:26", Tag::Error, "app", "y", &[], false);
        assert_eq!(a.find('|'), b.find('|'));
    }

    #[test]
    fn color_wraps_only_when_enabled() {
        let line = render_event("09:14:25", Tag::Ok, "app", "listo", &[], true);
        assert!(line.contains("\x1b[32m[ OK  ]\x1b[0m"));
        let line = render_event("09:14:25", Tag::Error, "app", "falló", &[], true);
        assert!(line.contains("\x1b[31m[ERROR]\x1b[0m"));
    }

    #[test]
    fn details_go_on_indented_lines() {
        let details = vec![("error".to_string(), "connection refused".to_string())];
        let text = render_event("09:14:25", Tag::Error, "app", "No se pudo", &details, false);
        let second = text.lines().nth(1).unwrap();
        assert_eq!(
            second,
            format!("{}+- motivo: connection refused", " ".repeat(17))
        );
    }

    #[test]
    fn banner_is_ascii_and_rectangular() {
        let text = banner();
        assert!(text.is_ascii());
        assert!(text.contains("CASHFLOW POS - DEV MODE"));
        let widths: Vec<usize> = text.lines().map(|line| line.chars().count()).collect();
        assert!(widths.iter().all(|width| *width == BANNER_WIDTH + 2));
    }

    // ---- rachas repetidas ----

    fn candidate<'a>(key: &'a str, collapsible: bool, time: &'a str) -> Candidate<'a> {
        Candidate {
            key,
            collapsible,
            tag: Tag::Error,
            target: "audit::failure",
            time,
        }
    }

    fn secs(base: Instant, n: u64) -> Instant {
        base + Duration::from_secs(n)
    }

    #[test]
    fn repeated_errors_are_suppressed_then_summarized_by_next_event() {
        let t0 = Instant::now();
        let mut guard = RepeatGuard::default();

        let first = guard.observe(&candidate("e", true, "10:00:00"), t0);
        assert!(!first.suppress && first.flush.is_none());

        for n in 1..=5 {
            let again = guard.observe(&candidate("e", true, "10:00:03"), secs(t0, n / 2));
            assert!(again.suppress && again.flush.is_none());
        }

        let other = guard.observe(&candidate("otro", true, "10:00:04"), secs(t0, 3));
        assert!(!other.suppress);
        let summary = other.flush.expect("la racha debe resumirse");
        assert_eq!(summary.total, 6);
        assert_eq!(summary.last_time, "10:00:03");
        assert_eq!(summary.target, "audit::failure");
    }

    #[test]
    fn a_single_error_never_produces_a_summary() {
        let t0 = Instant::now();
        let mut guard = RepeatGuard::default();
        guard.observe(&candidate("e", true, "10:00:00"), t0);
        let other = guard.observe(&candidate("otro", true, "10:00:01"), secs(t0, 1));
        assert!(other.flush.is_none());
        assert!(guard.take_expired(secs(t0, 60)).is_none());
    }

    #[test]
    fn same_error_after_the_window_prints_again() {
        let t0 = Instant::now();
        let mut guard = RepeatGuard::default();
        guard.observe(&candidate("e", true, "10:00:00"), t0);
        guard.observe(&candidate("e", true, "10:00:01"), secs(t0, 1));

        let later = guard.observe(&candidate("e", true, "10:01:00"), secs(t0, 60));
        assert!(!later.suppress);
        assert_eq!(later.flush.map(|summary| summary.total), Some(2));
    }

    #[test]
    fn streak_is_flushed_by_timer_only_after_the_window() {
        let t0 = Instant::now();
        let mut guard = RepeatGuard::default();
        guard.observe(&candidate("e", true, "10:00:00"), t0);
        guard.observe(&candidate("e", true, "10:00:02"), secs(t0, 2));
        guard.observe(&candidate("e", true, "10:00:03"), secs(t0, 3));

        assert!(guard.take_expired(secs(t0, 5)).is_none());
        let summary = guard.take_expired(secs(t0, 9)).expect("ventana vencida");
        assert_eq!(summary.total, 3);
        assert_eq!(summary.last_time, "10:00:03");
        assert!(guard.take_expired(secs(t0, 20)).is_none());
    }

    #[test]
    fn non_collapsible_events_are_never_suppressed_and_end_the_streak() {
        let t0 = Instant::now();
        let mut guard = RepeatGuard::default();
        guard.observe(&candidate("e", true, "10:00:00"), t0);
        guard.observe(&candidate("e", true, "10:00:01"), secs(t0, 1));

        let info = guard.observe(&candidate("e", false, "10:00:02"), secs(t0, 2));
        assert!(!info.suppress);
        assert_eq!(info.flush.map(|summary| summary.total), Some(2));

        let info_again = guard.observe(&candidate("e", false, "10:00:03"), secs(t0, 3));
        assert!(!info_again.suppress);
    }

    #[test]
    fn repeat_key_distinguishes_message_and_details() {
        let none: Vec<(String, String)> = vec![];
        let with = vec![("error".to_string(), "x".to_string())];
        let a = repeat_key(Tag::Error, "t", "m", &none);
        assert_eq!(a, repeat_key(Tag::Error, "t", "m", &none));
        assert_ne!(a, repeat_key(Tag::Error, "t", "otro", &none));
        assert_ne!(a, repeat_key(Tag::Error, "t", "m", &with));
        assert_ne!(a, repeat_key(Tag::Warn, "t", "m", &none));
    }

    #[test]
    fn summary_line_matches_event_layout() {
        let summary = RepeatSummary {
            tag: Tag::Error,
            target: "audit::failure".to_string(),
            total: 6,
            last_time: "10:00:03".to_string(),
            span: Duration::from_millis(2500),
        };
        let line = render_repeat(&summary, false);
        assert!(line.starts_with("10:00:03 [ERROR] audit::failure"));
        assert!(line.contains("| ^ x6 en total durante 2.5 s"));
        assert!(line.is_ascii() && line.ends_with('\n'));
        let event = render_event("10:00:03", Tag::Error, "audit::failure", "x", &[], false);
        assert_eq!(line.find('|'), event.find('|'));
        assert!(render_repeat(&summary, true).contains("\x1b[31m[ERROR]\x1b[0m"));
    }

    // ---- panel de arranque ----

    fn sample_info(db: Result<DbHealth, String>) -> StartupInfo {
        StartupInfo {
            app_version: "0.1.0",
            profile: "debug",
            log_filter: DEFAULT_FILTER.to_string(),
            db,
            pool_size: 2,
            pool_idle: 2,
            pool_max: 10,
            startup: Duration::from_millis(1234),
        }
    }

    fn healthy() -> Result<DbHealth, String> {
        Ok(DbHealth {
            database: "pos_db".to_string(),
            server_version: "PostgreSQL 18.0 on x86_64-pc-windows, compiled by msvc".to_string(),
            latency: Duration::from_micros(4200),
        })
    }

    #[test]
    fn short_pg_version_keeps_name_and_number() {
        assert_eq!(
            short_pg_version("PostgreSQL 18.0 on x86_64-pc-windows, compiled by msvc"),
            "PostgreSQL 18.0"
        );
        assert_eq!(short_pg_version(""), "version desconocida");
    }

    #[test]
    fn short_pg_version_drops_trailing_comma() {
        assert_eq!(
            short_pg_version("PostgreSQL 16.13, compiled by Visual C++ build 1944"),
            "PostgreSQL 16.13"
        );
    }

    #[test]
    fn effective_filter_prefers_rust_log() {
        assert_eq!(
            effective_filter(Some("warn,pos_lib=debug")),
            "warn,pos_lib=debug"
        );
        assert_eq!(effective_filter(Some("  ")), DEFAULT_FILTER);
        assert_eq!(effective_filter(None), DEFAULT_FILTER);
    }

    #[test]
    fn latency_is_shown_with_one_decimal() {
        assert_eq!(format_millis(Duration::from_micros(4200)), "4.2 ms");
    }

    #[test]
    fn startup_panel_is_ascii_and_has_all_rows() {
        let text = render_startup_panel(&sample_info(healthy()), false);
        assert!(text.is_ascii() && !text.contains('\x1b'));
        for needle in [
            "App",
            "Base de datos",
            "Pool",
            "Migraciones",
            "Arranque",
            "Developer",
            "Diagnostico",
            "v0.1.0, debug",
            "pos_db, PostgreSQL 18.0, 4.2 ms",
            "2 abiertas, 2 libres, max 10",
            "1.2 s",
            "/developer",
            "RUST_LOG=warn,pos_lib=debug",
        ] {
            assert!(text.contains(needle), "falta: {needle}");
        }
    }

    #[test]
    fn startup_panel_values_share_one_column() {
        let text = render_startup_panel(&sample_info(healthy()), false);
        let col = 2 + PANEL_LABEL_WIDTH + PANEL_TAG_SLOT;
        for needle in [
            "v0.1.0",
            "pos_db",
            "2 abiertas",
            "aplicadas",
            "1.2 s",
            "Ajustes",
            "RUST_LOG",
        ] {
            let line = text.lines().find(|line| line.contains(needle)).unwrap();
            assert_eq!(line.find(needle), Some(col), "columna de: {needle}");
        }
    }

    #[test]
    fn failed_health_query_shows_warn_and_redacts() {
        let info = sample_info(Err("fallo postgres://usuario:clave@host/db".to_string()));
        let text = render_startup_panel(&info, false);
        let line = text
            .lines()
            .find(|line| line.contains("Base de datos"))
            .unwrap();
        assert!(line.contains("[WARN ]"));
        assert!(!text.contains("clave"));
        assert!(text.contains("Migraciones"));
    }

    #[test]
    fn startup_panel_colors_only_when_enabled() {
        let info = sample_info(healthy());
        assert!(render_startup_panel(&info, true).contains("\x1b[32m[ OK  ]\x1b[0m"));
        assert!(!render_startup_panel(&info, false).contains('\x1b'));
    }
}

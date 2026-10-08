//! Formato de logs para desarrollo.

use std::fmt::{self, Write as _};
use std::io::IsTerminal;

use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer, Registry};

pub const DEFAULT_FILTER: &str = "warn,pos_lib=info,sqlx=warn";

const TARGET_WIDTH: usize = 16;
const LABEL_WIDTH: usize = 7;
const BANNER_WIDTH: usize = 54;
const MAX_DETAIL_CHARS: usize = 300;
const DETAIL_PREFIX: &str = "+- ";
const DIM: &str = "90";

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

pub fn short_target(target: &str, verbose: bool) -> String {
    if verbose {
        return target.to_string();
    }
    if target == "pos_lib" {
        return "app".to_string();
    }
    target
        .strip_prefix("pos_lib::")
        .unwrap_or(target)
        .to_string()
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

pub fn clean_value(value: &str) -> String {
    let flat = redact_credentials(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if flat.chars().count() > MAX_DETAIL_CHARS {
        let mut cut: String = flat.chars().take(MAX_DETAIL_CHARS).collect();
        cut.push_str("...");
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
    let padded_target = format!("{:<width$}", target, width = TARGET_WIDTH);
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
struct EventFields {
    message: String,
    ok: bool,
    details: Vec<(String, String)>,
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

        let line = render_event(&time, tag, &target, &message, &fields.details, self.color);
        writer.write_str(&line)
    }
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

pub fn init() {
    let stdout_is_terminal = std::io::stdout().is_terminal();
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    let color = colors_enabled(stdout_is_terminal, no_color) && enable_ansi();

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let verbose = is_verbose(<EnvFilter as Layer<Registry>>::max_level_hint(&filter));

    if cfg!(debug_assertions) && stdout_is_terminal {
        println!("{}", banner());
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .event_format(DevFormat { color, verbose })
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_target_strips_crate_prefix() {
        assert_eq!(
            short_target("pos_lib::db::connection", false),
            "db::connection"
        );
        assert_eq!(short_target("pos_lib", false), "app");
        assert_eq!(short_target("sqlx::query", false), "sqlx::query");
        assert_eq!(
            short_target("pos_lib::db::connection", true),
            "pos_lib::db::connection"
        );
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
        assert_eq!(clean_value(&long).chars().count(), MAX_DETAIL_CHARS + 3);
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
}

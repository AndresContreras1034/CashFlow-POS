//! Layer de `tracing` que alimenta Developer Mode con WARN y ERROR de `pos_lib`.

use std::sync::Arc;

use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use super::models::{DeveloperEvent, DeveloperLevel, DeveloperOutcome};
use super::state::DeveloperModeState;
use crate::logging::{short_target, EventFields};

/// Solo WARN y ERROR de este crate. INFO/DEBUG y dependencias no entran.
pub fn is_captured(target: &str, level: &Level) -> bool {
    (target == "pos_lib" || target.starts_with("pos_lib::")) && *level <= Level::WARN
}

fn wants(meta: &Metadata<'_>) -> bool {
    is_captured(meta.target(), meta.level())
}

struct DeveloperLayer {
    state: Arc<DeveloperModeState>,
}

impl<S: Subscriber> Layer<S> for DeveloperLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if !self.state.is_enabled() {
            return;
        }
        let meta = event.metadata();
        let mut fields = EventFields::default();
        event.record(&mut fields);

        // Solo `message` y, si existe, `error`. Ningún otro campo se guarda.
        let error = fields
            .details
            .iter()
            .find(|(name, _)| name == "error")
            .map(|(_, value)| value.as_str());
        let raw = match (fields.message.is_empty(), error) {
            (false, Some(error)) => format!("{}: {}", fields.message, error),
            (false, None) => fields.message.clone(),
            (true, Some(error)) => error.to_string(),
            (true, None) => String::new(),
        };

        let (level, outcome) = if *meta.level() == Level::ERROR {
            (DeveloperLevel::Error, DeveloperOutcome::Failure)
        } else {
            (DeveloperLevel::Warn, DeveloperOutcome::Warn)
        };
        let target = short_target(meta.target(), false);

        self.state.push(DeveloperEvent::new(
            None, &target, &target, outcome, level, &raw,
        ));
    }
}

pub fn build<S>(state: Arc<DeveloperModeState>) -> impl Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    DeveloperLayer { state }.with_filter(filter_fn(wants))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    fn run(state: &Arc<DeveloperModeState>, f: impl FnOnce()) {
        let subscriber = tracing_subscriber::registry().with(build(state.clone()));
        tracing::subscriber::with_default(subscriber, f);
    }

    fn enabled() -> Arc<DeveloperModeState> {
        let state = Arc::new(DeveloperModeState::new());
        state.set_enabled(true);
        state
    }

    #[test]
    fn target_and_level_rules() {
        assert!(is_captured("pos_lib", &Level::WARN));
        assert!(is_captured("pos_lib::db::connection", &Level::ERROR));
        assert!(!is_captured("pos_lib::db", &Level::INFO));
        assert!(!is_captured("pos_lib::db", &Level::DEBUG));
        assert!(!is_captured("sqlx::query", &Level::ERROR));
        assert!(!is_captured("pos_lib_other", &Level::ERROR));
    }

    #[test]
    fn captures_warn_and_error_with_error_field() {
        let state = enabled();
        run(&state, || {
            tracing::warn!("aviso");
            tracing::error!(error = %"conexión rechazada", "No se pudo");
        });
        let events = state.snapshot();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "No se pudo: conexión rechazada");
        assert_eq!(events[0].level, DeveloperLevel::Error);
        assert_eq!(events[0].outcome, DeveloperOutcome::Failure);
        assert_eq!(events[1].level, DeveloperLevel::Warn);
        assert!(events[0].correlation_id.is_none() && events[0].duration_ms.is_none());
    }

    #[test]
    fn ignores_info_and_foreign_targets() {
        let state = enabled();
        run(&state, || {
            tracing::info!("listo");
            tracing::error!(target: "sqlx::query", "ruido");
        });
        assert_eq!(state.status().count, 0);
    }

    #[test]
    fn ignores_everything_while_disabled() {
        let state = Arc::new(DeveloperModeState::new());
        run(&state, || tracing::error!("x"));
        assert_eq!(state.status().count, 0);
    }

    #[test]
    fn redacts_credentials_and_ignores_other_fields() {
        let state = enabled();
        run(&state, || {
            tracing::error!(token = "secreto123", error = %"postgres://u:clave@h/db", "falló");
        });
        let message = &state.snapshot()[0].message;
        assert!(!message.contains("clave"));
        assert!(!message.contains("secreto123"));
    }
}

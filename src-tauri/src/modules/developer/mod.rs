//! Developer Mode: diagnóstico técnico temporal, solo en memoria.
//!
//! No es Audit: no persiste nada ni guarda datos de negocio.

pub mod handlers;
pub mod layer;
pub mod models;
pub mod router;
pub mod state;

use uuid::Uuid;

use models::{DeveloperEvent, DeveloperLevel, DeveloperOutcome};
use state::{shared, DeveloperModeState};

/// Falla técnica de un comando (la llama `record_failure`).
/// `correlation_id` solo viene si el evento de Audit realmente se insertó.
pub fn capture_failure(
    module: &str,
    action: &str,
    kind: &str,
    message: &str,
    correlation_id: Option<Uuid>,
) {
    capture_failure_in(&shared(), module, action, kind, message, correlation_id);
}

pub fn capture_failure_in(
    state: &DeveloperModeState,
    module: &str,
    action: &str,
    kind: &str,
    message: &str,
    correlation_id: Option<Uuid>,
) {
    if !state.is_enabled() {
        return;
    }
    state.push(DeveloperEvent::new(
        correlation_id,
        &format!("{module}.{action}"),
        module,
        DeveloperOutcome::Failure,
        DeveloperLevel::Error,
        &format!("{kind}: {message}"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> DeveloperModeState {
        let state = DeveloperModeState::new();
        state.set_enabled(true);
        state
    }

    #[test]
    fn failure_is_redacted_capped_and_has_no_duration() {
        let state = enabled();
        let long = format!(
            "x postgres://usuario:clave@localhost/pos {}",
            "y".repeat(500)
        );
        capture_failure_in(&state, "sales", "create", "database", &long, None);
        let events = state.snapshot();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, "sales.create");
        assert_eq!(events[0].target, "sales");
        assert!(!events[0].message.contains("clave"));
        assert_eq!(events[0].message.chars().count(), 300);
        assert!(events[0].duration_ms.is_none() && events[0].correlation_id.is_none());
    }

    #[test]
    fn failure_keeps_shared_correlation_id() {
        let state = enabled();
        let id = Uuid::new_v4();
        capture_failure_in(&state, "cash", "open", "internal", "falló", Some(id));
        assert_eq!(state.snapshot()[0].correlation_id, Some(id));
    }

    #[test]
    fn failure_is_ignored_while_disabled() {
        let state = DeveloperModeState::new();
        capture_failure_in(&state, "cash", "open", "internal", "x", None);
        assert_eq!(state.status().count, 0);
    }
}

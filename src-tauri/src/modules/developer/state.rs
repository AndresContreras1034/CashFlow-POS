//! Estado compartido de Developer Mode: interruptor + buffer circular en memoria.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use super::models::{DeveloperEvent, DeveloperStatus};

pub const CAPACITY: usize = 500;

pub struct DeveloperModeState {
    enabled: AtomicBool,
    buffer: Mutex<VecDeque<DeveloperEvent>>,
}

impl Default for DeveloperModeState {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperModeState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            buffer: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    // Sección crítica corta, sin `.await` ni `tracing` dentro del lock.
    fn lock(&self) -> MutexGuard<'_, VecDeque<DeveloperEvent>> {
        self.buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Guarda el evento solo si el modo está activo. Descarta el más antiguo
    /// al llegar a `CAPACITY`.
    pub fn push(&self, event: DeveloperEvent) {
        if !self.is_enabled() {
            return;
        }
        let mut buffer = self.lock();
        if buffer.len() >= CAPACITY {
            buffer.pop_front();
        }
        buffer.push_back(event);
    }

    /// Más reciente primero.
    pub fn snapshot(&self) -> Vec<DeveloperEvent> {
        self.lock().iter().rev().cloned().collect()
    }

    pub fn clear(&self) {
        self.lock().clear();
    }

    pub fn status(&self) -> DeveloperStatus {
        DeveloperStatus {
            enabled: self.is_enabled(),
            count: self.lock().len(),
            capacity: CAPACITY,
        }
    }
}

static SHARED: OnceLock<Arc<DeveloperModeState>> = OnceLock::new();

/// Instancia única de la app. Se registra en Tauri con `.manage(...)` y la
/// usan el Layer de tracing y `record_failure`.
pub fn shared() -> Arc<DeveloperModeState> {
    SHARED
        .get_or_init(|| Arc::new(DeveloperModeState::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::developer::models::{DeveloperLevel, DeveloperOutcome};

    fn event(message: &str) -> DeveloperEvent {
        DeveloperEvent::new(
            None,
            "app",
            "app",
            DeveloperOutcome::Warn,
            DeveloperLevel::Warn,
            message,
        )
    }

    #[test]
    fn starts_disabled_and_ignores_events() {
        let state = DeveloperModeState::new();
        assert!(!state.is_enabled());
        state.push(event("x"));
        assert_eq!(state.status().count, 0);
    }

    #[test]
    fn keeps_at_most_capacity_dropping_oldest() {
        let state = DeveloperModeState::new();
        state.set_enabled(true);
        for i in 0..(CAPACITY + 100) {
            state.push(event(&format!("e{i}")));
        }
        let events = state.snapshot();
        assert_eq!(events.len(), CAPACITY);
        assert_eq!(events[0].message, format!("e{}", CAPACITY + 99));
        assert_eq!(events[CAPACITY - 1].message, "e100");
    }

    #[test]
    fn snapshot_is_newest_first_and_clear_empties() {
        let state = DeveloperModeState::new();
        state.set_enabled(true);
        state.push(event("a"));
        state.push(event("b"));
        assert_eq!(state.snapshot()[0].message, "b");
        state.clear();
        assert_eq!(state.status().count, 0);
        assert!(state.is_enabled());
    }

    #[test]
    fn disabling_keeps_buffer_but_stops_capture() {
        let state = DeveloperModeState::new();
        state.set_enabled(true);
        state.push(event("a"));
        state.set_enabled(false);
        state.push(event("b"));
        assert_eq!(state.status().count, 1);
    }

    #[test]
    fn event_new_cleans_message() {
        let e = event("fallo postgres://usuario:clave@host/db");
        assert!(!e.message.contains("clave"));
        assert!(e.duration_ms.is_none());
    }
}

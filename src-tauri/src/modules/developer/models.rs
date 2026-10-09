//! Contratos de Developer Mode.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::logging::clean_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeveloperOutcome {
    Success,
    Warn,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeveloperLevel {
    Warn,
    Error,
}

/// Evento técnico temporal. Solo se construye con `new`, que limpia todo el
/// texto (redacción de credenciales y tope de 300 caracteres).
#[derive(Debug, Clone, Serialize)]
pub struct DeveloperEvent {
    pub id: Uuid,
    pub correlation_id: Option<Uuid>,
    pub operation: String,
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub outcome: DeveloperOutcome,
    pub level: DeveloperLevel,
    pub message: String,
}

impl DeveloperEvent {
    pub fn new(
        correlation_id: Option<Uuid>,
        operation: &str,
        target: &str,
        outcome: DeveloperOutcome,
        level: DeveloperLevel,
        message: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id,
            operation: clean_value(operation),
            target: clean_value(target),
            timestamp: Utc::now(),
            duration_ms: None,
            outcome,
            level,
            message: clean_value(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperStatus {
    pub enabled: bool,
    pub count: usize,
    pub capacity: usize,
}

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::modules::audit::models::{AuditCategory, AuditModule, AuditOutcome};
use crate::modules::inventory::dto::SortDir;

// ============================================================
// ESCRITURA
// ============================================================

/// Evento a insertar en `audit_events`.
///
/// A propósito NO deriva `Deserialize`: los eventos los construye el
/// backend, nunca llegan desde el frontend.
#[derive(Debug, Clone)]
pub struct NewAuditEvent {
    pub correlation_id: Option<Uuid>,
    pub category: AuditCategory,
    pub module: AuditModule,
    pub action: String,
    pub outcome: AuditOutcome,
    pub actor: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub summary: String,
    pub changes: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
    pub error_message: Option<String>,
}

// ============================================================
// FILTROS / BÚSQUEDA
// ============================================================

/// Filtros del listado de eventos (futuro Audit Explorer).
///
/// - `search` busca (ILIKE) en summary, action, entity_id y error_message.
/// - `date_from` / `date_to` son días inclusivos, con la misma semántica
///   que el resto del repositorio (día según la zona de la sesión de BD).
/// - `sort_dir` ordena por fecha; por defecto `desc` (más reciente primero).
#[derive(Debug, Deserialize, Default)]
pub struct AuditEventFilterDto {
    pub module: Option<AuditModule>,
    pub category: Option<AuditCategory>,
    pub outcome: Option<AuditOutcome>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub search: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub sort_dir: Option<SortDir>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

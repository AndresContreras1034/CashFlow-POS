use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================
// ENUMS (espejo de los tipos de la migración 012)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Business,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_module", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditModule {
    Sales,
    Inventory,
    Cash,
    Stocktake,
    Settings,
    Import,
    Licensing,
    Billing,
    System,
    Debts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_outcome", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
}

// ============================================================
// EVENTO DE AUDITORÍA
// ============================================================

/// Fila de `audit_events`. La tabla es append-only: este struct solo se
/// lee; los eventos nuevos se insertarán con un DTO propio (etapa 2).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditEvent {
    pub id: i64,
    pub created_at: DateTime<Utc>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_as_snake_case_like_the_database_values() {
        assert_eq!(
            serde_json::to_string(&AuditModule::Stocktake).unwrap(),
            "\"stocktake\""
        );
        assert_eq!(
            serde_json::to_string(&AuditCategory::Business).unwrap(),
            "\"business\""
        );
        assert_eq!(
            serde_json::to_string(&AuditOutcome::Failure).unwrap(),
            "\"failure\""
        );
    }
}

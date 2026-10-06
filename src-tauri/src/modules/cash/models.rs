use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "cash_movement_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CashMovementType {
    ManualIn,
    ManualOut,
    SaleIn,
    SaleOut,
}

impl CashMovementType {
    /// Solo estos dos se pueden registrar manualmente desde el comando de Tauri.
    /// SaleIn/SaleOut quedan reservados para cuando exista modules::sales.
    pub fn is_manual(&self) -> bool {
        matches!(
            self,
            CashMovementType::ManualIn | CashMovementType::ManualOut
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CashSession {
    pub id: i32,
    pub opening_amount: i64,
    pub expected_amount: Option<i64>,
    pub counted_amount: Option<i64>,
    pub difference: Option<i64>,
    pub status: String,
    pub opened_by: String,
    pub closed_by: Option<String>,
    pub opening_notes: Option<String>,
    pub closing_notes: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl CashSession {
    pub fn is_open(&self) -> bool {
        self.status == "open"
    }
}

/// Sesión abierta junto con los totales acumulados hasta el momento —
/// usado para mostrar el estado en vivo de la caja sin tener que cerrarla.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CashSessionWithTotals {
    pub id: i32,
    pub opening_amount: i64,
    pub status: String,
    pub opened_by: String,
    pub opening_notes: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub total_in: i64,
    pub total_out: i64,
    pub current_balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CashMovement {
    pub id: i32,
    pub session_id: i32,
    pub movement_type: CashMovementType,
    pub amount: i64,
    pub sale_id: Option<i32>,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

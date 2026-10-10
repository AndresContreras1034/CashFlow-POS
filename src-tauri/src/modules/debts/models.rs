use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================
// ENUMS (espejo de los tipos de la migración 014)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "debt_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DebtCategory {
    Maintenance,
    BusinessOperation,
    FixedServices,
    OtherObligations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "debt_nature", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DebtNature {
    Operating,
    FinancialLoan,
    Tax,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "debt_payment_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DebtPaymentMethod {
    Cash,
    Card,
    Transfer,
    Check,
    Other,
}

/// Estado financiero. Lo calcula la vista `debt_overview`; nunca se guarda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "debt_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DebtStatus {
    Voided,
    Paid,
    Overdue,
    PartiallyPaid,
    Pending,
}

/// Urgencia del vencimiento, independiente del estado financiero.
/// Lo calcula la vista `debt_overview`; el frontend solo la muestra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "debt_urgency", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DebtUrgency {
    None,
    Ok,
    Alert,
    DueToday,
    Overdue,
}

// ============================================================
// FILAS
// ============================================================

/// Fila de `debts`. Sin saldo ni estado: eso vive en `DebtOverview`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Debt {
    pub id: i32,
    pub creditor_name: String,
    pub category: DebtCategory,
    pub subcategory: Option<String>,
    pub nature: DebtNature,
    pub concept: String,
    pub original_amount: i64,
    pub issued_on: NaiveDate,
    pub due_on: Option<NaiveDate>,
    pub document_ref: Option<String>,
    pub notes: Option<String>,
    pub idempotency_key: Uuid,
    pub voided_at: Option<DateTime<Utc>>,
    pub voided_by: Option<String>,
    pub void_reason: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fila de `debt_payments`. Un pago anulado permanece (con `voided_at`).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DebtPayment {
    pub id: i32,
    pub debt_id: i32,
    pub amount: i64,
    pub paid_on: NaiveDate,
    pub method: DebtPaymentMethod,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub idempotency_key: Uuid,
    pub voided_at: Option<DateTime<Utc>>,
    pub voided_by: Option<String>,
    pub void_reason: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// Fila de la vista `debt_overview`: la deuda con saldo, estado y urgencia
/// ya calculados por la base de datos.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DebtOverview {
    pub id: i32,
    pub creditor_name: String,
    pub category: DebtCategory,
    pub subcategory: Option<String>,
    pub nature: DebtNature,
    pub concept: String,
    pub original_amount: i64,
    pub issued_on: NaiveDate,
    pub due_on: Option<NaiveDate>,
    pub document_ref: Option<String>,
    pub notes: Option<String>,
    pub voided_at: Option<DateTime<Utc>>,
    pub voided_by: Option<String>,
    pub void_reason: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub paid_amount: i64,
    pub balance: i64,
    pub payment_count: i64,
    pub last_paid_on: Option<NaiveDate>,
    pub today: NaiveDate,
    /// Negativo = ya venció. `None` si la deuda no tiene vencimiento.
    pub days_until_due: Option<i32>,
    pub status: DebtStatus,
    pub urgency: DebtUrgency,
}
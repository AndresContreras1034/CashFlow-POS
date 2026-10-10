use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::debts::models::{DebtCategory, DebtNature, DebtPaymentMethod};

// ============================================================
// DEUDAS
// ============================================================

/// El repository recibe los textos ya normalizados (sin espacios sobrantes
/// y con `None` en lugar de vacío); eso es trabajo del service (Etapa 2).
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDebtDto {
    /// Lo genera el frontend al abrir el formulario; repetir el envío con la
    /// misma clave no crea una segunda deuda.
    pub idempotency_key: Uuid,
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
    pub created_by: Option<String>,
}

/// Reemplazo completo de los campos editables: lo que llega es lo que queda
/// (los opcionales en `None` se borran), igual que en Ajustes.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDebtDto {
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
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoidDebtDto {
    pub reason: String,
    pub voided_by: Option<String>,
}

// ============================================================
// PAGOS
// ============================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterPaymentDto {
    pub idempotency_key: Uuid,
    pub amount: i64,
    pub paid_on: NaiveDate,
    pub method: DebtPaymentMethod,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoidPaymentDto {
    pub reason: String,
    pub voided_by: Option<String>,
}
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::debts::models::{
    DebtCategory, DebtNature, DebtPaymentMethod, DebtStatus, DebtUrgency,
};

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

// ============================================================
// CONSULTAS
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    #[default]
    Desc,
}

/// Columnas por las que se puede ordenar el listado. Es una lista cerrada:
/// el texto del ORDER BY sale de aquí, nunca del cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DebtSortBy {
    /// Importe original.
    Amount,
    /// Fecha de emisión (la escribe el usuario).
    IssuedOn,
    /// Fecha de registro (la pone el sistema).
    #[default]
    CreatedAt,
    DueOn,
    Balance,
}

/// Filtros del listado. El resumen y las estadísticas por categoría usan el
/// mismo DTO (ignoran orden y paginación), así sus totales siempre coinciden
/// con lo que muestra el listado filtrado.
///
/// - `search` busca (ILIKE) en acreedor, concepto y referencia documental.
/// - Los tres rangos de fechas son independientes y inclusivos:
///   emisión, vencimiento y pago. El de pago devuelve las deudas con al menos
///   un pago VIGENTE dentro del rango.
/// - `urgencies` admite varias (p. ej. próximas a vencer y vencidas).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DebtFilterDto {
    pub search: Option<String>,
    pub category: Option<DebtCategory>,
    pub nature: Option<DebtNature>,
    pub status: Option<DebtStatus>,
    pub urgencies: Option<Vec<DebtUrgency>>,
    pub issued_from: Option<NaiveDate>,
    pub issued_to: Option<NaiveDate>,
    pub due_from: Option<NaiveDate>,
    pub due_to: Option<NaiveDate>,
    pub paid_from: Option<NaiveDate>,
    pub paid_to: Option<NaiveDate>,
    pub sort_by: Option<DebtSortBy>,
    pub sort_dir: Option<SortDir>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Paginación del listado de vencimientos.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DueDebtsFilterDto {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}
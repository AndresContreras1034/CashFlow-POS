use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::modules::cash::models::CashMovementType;

// ============================================================
// APERTURA / CIERRE DE TURNO
// ============================================================

#[derive(Debug, Deserialize)]
pub struct OpenSessionDto {
    pub opening_amount: i64,
    pub opening_notes: Option<String>,
    pub opened_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloseSessionDto {
    pub counted_amount: i64,
    pub closing_notes: Option<String>,
    pub closed_by: Option<String>,
}

// ============================================================
// MOVIMIENTOS
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateMovementDto {
    pub movement_type: CashMovementType,
    pub amount: i64,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    // Reservado para cuando exista modules::sales; no se usa todavía.
    pub sale_id: Option<i32>,
}

// ============================================================
// FILTROS / PAGINACIÓN
// ============================================================

#[derive(Debug, Deserialize, Default)]
pub struct CashSessionFilterDto {
    pub status: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CashMovementFilterDto {
    pub session_id: Option<i32>,
    pub movement_type: Option<String>,
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

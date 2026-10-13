use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCustomerDto {
    pub name: String,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

/// Reemplazo completo de los campos editables: los opcionales en `None` se borran.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCustomerDto {
    pub name: String,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetCustomerActiveDto {
    pub is_active: bool,
    pub updated_by: Option<String>,
}

/// `search` busca (ILIKE) en nombre, documento, teléfono y email.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CustomerFilterDto {
    pub search: Option<String>,
    pub is_active: Option<bool>,
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

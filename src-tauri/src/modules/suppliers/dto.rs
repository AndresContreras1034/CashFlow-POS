use serde::{Deserialize, Serialize};

/// Campos editables de un proveedor. Create y Update comparten la misma forma.
#[derive(Debug, Clone, Deserialize)]
pub struct SupplierFields {
    pub name: String,
    pub contact_name: Option<String>,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub payment_terms_days: Option<i32>,
    pub lead_time_days: Option<i32>,
    pub visit_days: Option<String>,
    /// `None` equivale a `normal`.
    pub priority: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSupplierDto {
    #[serde(flatten)]
    pub fields: SupplierFields,
    pub created_by: Option<String>,
}

/// Reemplazo completo de los campos editables: los opcionales en `None` se borran.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSupplierDto {
    #[serde(flatten)]
    pub fields: SupplierFields,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSupplierActiveDto {
    pub is_active: bool,
    pub updated_by: Option<String>,
}

/// `search` busca (ILIKE) en nombre, contacto, documento, teléfono, WhatsApp, email y ciudad.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SupplierFilterDto {
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub priority: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkSupplierVariantDto {
    pub variant_id: i32,
    pub supplier_sku: Option<String>,
    pub is_preferred: bool,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSupplierVariantDto {
    pub supplier_sku: Option<String>,
    pub is_preferred: bool,
    pub notes: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LinkableFilterDto {
    pub search: Option<String>,
    pub category_id: Option<i32>,
    pub only_unassigned: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BulkLinkDto {
    pub variant_ids: Vec<i32>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnlinkSupplierVariantDto {
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSupplierCategoriesDto {
    pub category_ids: Vec<i32>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SupplierVariantFilterDto {
    pub search: Option<String>,
    pub only_preferred: Option<bool>,
    pub category_id: Option<i32>,
    pub include_inactive: Option<bool>,
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

pub use super::restock::RestockFilterDto;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Supplier {
    pub id: i32,
    pub name: String,
    pub contact_name: Option<String>,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    /// Plazo de pago acordado, en días (0 = de contado).
    pub payment_terms_days: Option<i32>,
    /// Días que tarda en entregar un pedido.
    pub lead_time_days: Option<i32>,
    /// Texto libre: «Lunes y jueves».
    pub visit_days: Option<String>,
    /// `normal` | `preferred` | `urgent`.
    pub priority: String,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupplierOverview {
    pub id: i32,
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
    pub priority: String,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category_count: i64,
    pub variant_count: i64,
    pub low_stock_count: i64,
    pub out_of_stock_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupplierCategory {
    pub category_id: i32,
    pub category_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupplierVariantLink {
    pub id: i32,
    pub supplier_id: i32,
    pub variant_id: i32,
    pub supplier_sku: Option<String>,
    pub last_cost: Option<i64>,
    pub is_preferred: bool,
    pub last_purchase_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupplierVariantView {
    pub id: i32,
    pub supplier_id: i32,
    pub variant_id: i32,
    pub supplier_sku: Option<String>,
    pub last_cost: Option<i64>,
    pub is_preferred: bool,
    pub last_purchase_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub product_name: String,
    pub attributes: serde_json::Value,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category_id: i32,
    pub category_name: String,
    pub stock: i32,
    pub stock_min: i32,
    pub stock_status: String,
    pub variant_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VariantSupplier {
    pub id: i32,
    pub supplier_id: i32,
    pub supplier_name: String,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub supplier_sku: Option<String>,
    pub last_cost: Option<i64>,
    pub last_purchase_at: Option<DateTime<Utc>>,
    pub is_preferred: bool,
}

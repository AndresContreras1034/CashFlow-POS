use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Customer {
    pub id: i32,
    pub name: String,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// El cliente más sus estadísticas de compra (solo ventas completadas).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CustomerOverview {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub customer: Customer,
    pub sales_count: i64,
    pub total_spent: i64,
    pub last_purchase_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CustomerName {
    pub id: i32,
    pub name: String,
}

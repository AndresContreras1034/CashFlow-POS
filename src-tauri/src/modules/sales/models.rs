use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    Transfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "sale_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SaleStatus {
    Completed,
    Cancelled,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Sale {
    pub id: i32,
    pub customer_id: Option<i32>,
    pub subtotal: i64,
    pub tax: i64,
    pub discount: i64,
    pub total: i64,
    pub cash_received: Option<i64>,
    pub change_given: Option<i64>,
    pub status: SaleStatus,
    pub notes: Option<String>,
    pub courtesy_reason: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SaleItem {
    pub id: i32,
    pub sale_id: i32,
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_price: i64,
    pub discount: i64,
    pub tax: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SalePayment {
    pub id: i32,
    pub sale_id: i32,
    pub method: PaymentMethod,
    pub amount: i64,
}

/// Venta completa con ítems y pagos — se ensambla a mano con 3 queries
/// en el repository, no es un FromRow directo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleDetail {
    #[serde(flatten)]
    pub sale: Sale,
    pub items: Vec<SaleItem>,
    pub payments: Vec<SalePayment>,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "stocktake_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StocktakeStatus {
    Counting,
    Applied,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stocktake {
    pub id: i32,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub status: StocktakeStatus,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub total_lines: i64,
    pub counted_lines: i64,
}

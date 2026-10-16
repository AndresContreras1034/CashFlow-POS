use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CommercialEvent {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub rule: String,
    pub event_date: Option<NaiveDate>,
    pub month: Option<i16>,
    pub day: Option<i16>,
    pub weekday: Option<i16>,
    pub nth: Option<i16>,
    pub prep_days: i16,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommercialEventInput {
    pub name: String,
    pub kind: String,
    pub rule: String,
    pub event_date: Option<NaiveDate>,
    pub month: Option<i16>,
    pub day: Option<i16>,
    pub weekday: Option<i16>,
    pub nth: Option<i16>,
    pub prep_days: i16,
    pub is_active: bool,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpcomingCommercialEvent {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub date: NaiveDate,
    pub in_days: i64,
    pub prep_days: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesPace {
    pub as_of: NaiveDate,
    pub days: u16,
    pub actual_cents: i64,
    pub baseline_cents: Option<i64>,
    pub vs_baseline_bps: Option<i64>,
    pub status: String,
    pub year_ago_date: Option<NaiveDate>,
}

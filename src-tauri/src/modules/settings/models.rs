use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSettings {
    pub id: i16,
    pub business_name: String,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub currency: String,
    /// Visible currency precision (0 or 2); stored amounts remain in hundredths.
    pub currency_decimals: i16,
    pub tax_rate_bps: i32,
    pub tax_name: String,
    pub timezone: String,
    pub ticket_header: Option<String>,
    pub ticket_footer: Option<String>,
    pub show_logo: bool,
    pub show_tax_id: bool,
    pub show_address: bool,
    pub show_phone: bool,
    pub show_cashier: bool,
    pub show_tax_breakdown: bool,
    pub show_discounts: bool,
    pub show_payment_method: bool,
    pub low_stock_default_threshold: i32,
    pub logo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AppSettings {
    /// IVA como porcentaje legible (1900 -> 19.0)
    pub fn tax_rate_percent(&self) -> f64 {
        self.tax_rate_bps as f64 / 100.0
    }
}

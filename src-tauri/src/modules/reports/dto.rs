use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::modules::sales::models::PaymentMethod;

fn default_granularity() -> Option<Granularity> {
    Some(Granularity::Day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Granularity {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportFilter {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    #[serde(default = "default_granularity")]
    pub granularity: Option<Granularity>,
    #[serde(default)]
    pub compare_previous: bool,
    pub category_id: Option<i32>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    pub current: i64,
    pub previous: Option<i64>,
    pub change_bps: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesKpis {
    pub tickets: i64,
    pub gross_sales: i64,
    pub discount_total: i64,
    pub tax_total: i64,
    pub net_sales: i64,
    pub avg_ticket: i64,
    pub cogs: i64,
    pub gross_profit: i64,
    pub margin_bps: i32,
    pub units_sold: i64,
    pub items_without_cost: i64,
    pub cancelled_count: i64,
    pub refunded_count: i64,
    pub courtesy_count: i64,
    pub courtesy_cost: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardKpis {
    pub range: DateRange,
    pub previous_range: Option<DateRange>,
    pub current: SalesKpis,
    pub previous: Option<SalesKpis>,
    pub gross_sales_delta: Delta,
    pub gross_profit_delta: Delta,
    pub tickets_delta: Delta,
    pub avg_ticket_delta: Delta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub bucket: NaiveDate,
    pub tickets: i64,
    pub gross_sales: i64,
    pub gross_profit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentMethodStat {
    pub method: PaymentMethod,
    pub tickets: i64,
    pub amount: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub weekday: u8,
    pub hour: u8,
    pub tickets: i64,
    pub gross_sales: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopProduct {
    pub product_id: i32,
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: serde_json::Value,
    pub units: i64,
    pub revenue: i64,
    pub profit: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySales {
    pub category_id: i32,
    pub category_name: String,
    pub units: i64,
    pub revenue: i64,
    pub profit: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleProduct {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: serde_json::Value,
    pub stock: i32,
    pub stock_value_at_cost: i64,
    pub days_since_last_sale: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::{Granularity, ReportFilter};

    #[test]
    fn report_filter_defaults_to_daily_without_comparison() {
        let filter: ReportFilter =
            serde_json::from_str(r#"{"date_from":"2026-10-01","date_to":"2026-10-02"}"#)
                .expect("deserialize filter with defaults");

        assert_eq!(filter.granularity, Some(Granularity::Day));
        assert!(!filter.compare_previous);
        assert_eq!(filter.category_id, None);
        assert_eq!(filter.limit, None);
    }
}

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::modules::debts::models::{DebtCategory, DebtUrgency};
use crate::modules::inventory::models::StockStatus;
use crate::modules::sales::models::PaymentMethod;

fn default_granularity() -> Option<Granularity> {
    Some(Granularity::Day)
}

fn default_rank_by() -> Option<RankBy> {
    Some(RankBy::Revenue)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Granularity {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankBy {
    Units,
    Revenue,
    Profit,
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
    #[serde(default = "default_rank_by")]
    pub rank_by: Option<RankBy>,
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
    pub uncosted_tickets: i64,
    pub profit_base_net_sales: i64,
    pub courtesy_value: i64,
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
    /// Non-additive: a mixed-payment sale is counted once for each method it uses.
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
    pub gross_revenue: i64,
    pub revenue: i64,
    pub profit: i64,
    pub margin_bps: Option<i32>,
    pub uncosted_units: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySales {
    pub category_id: i32,
    pub category_name: String,
    pub units: i64,
    pub gross_revenue: i64,
    pub revenue: i64,
    pub profit: i64,
    pub margin_bps: Option<i32>,
    pub uncosted_units: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleProduct {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: serde_json::Value,
    pub stock: i32,
    pub stock_value_at_cost: i64,
    /// True when the inventory value is understated because no unit cost is recorded.
    pub cost_missing: bool,
    pub days_since_last_sale: Option<i64>,
    pub total_value_at_cost: i64,
    pub total_rows: i64,
    pub total_without_cost: i64,
}

/// Current stock and cost snapshot; date filters are intentionally ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryValuation {
    pub value_at_cost: i64,
    /// Mirrors inventory's existing valuation: variants without a recorded cost are excluded.
    pub potential_sale_value: i64,
    pub potential_margin_bps: Option<i32>,
    /// Active variants with cost > 0 regardless of stock, matching inventory's inherited semantics.
    pub variants_valued: i64,
    pub variants_without_cost: i64,
    pub negative_stock_variants: i64,
    pub units_in_stock: i64,
    pub by_category: Vec<CategoryValuation>,
}

/// Current stock snapshot by category; sales dates do not affect these values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryValuation {
    pub category_id: i32,
    pub category_name: String,
    pub units: i64,
    pub value_at_cost: i64,
    pub potential_sale_value: i64,
    pub share_bps: i32,
    pub variants_without_cost: i64,
}

/// Current inventory snapshot compared with cost of completed non-courtesy sales in the range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryTurnover {
    pub cogs: i64,
    pub inventory_value_at_cost: i64,
    pub turnover_bps: Option<i32>,
    pub days_of_inventory: Option<i64>,
    pub days_of_inventory_reason: Option<super::inventory_days::InsufficientReason>,
    pub range_days: i64,
    pub uncosted_items: i64,
}

/// Current stock snapshot with sales demand calculated over the requested date range.
/// Out-of-stock rows sort by units sold DESC, then low-stock rows by days of cover
/// ASC NULLS LAST; variant_id is the deterministic tie-breaker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockAlert {
    pub variant_id: i32,
    pub product_name: String,
    pub attributes: serde_json::Value,
    pub category_name: String,
    pub stock: i32,
    pub stock_min: i32,
    pub status: StockStatus,
    pub units_sold: i64,
    pub days_of_cover: Option<i64>,
    /// Expected demand over the selected range, less current stock, floored at zero.
    pub suggested_reorder: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopCustomer {
    pub customer_id: i32,
    pub customer_name: String,
    pub sales_count: i64,
    pub gross_sales: i64,
    pub revenue: i64,
    pub avg_ticket: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerSalesSplit {
    pub customer_id: i32,
    pub customer_name: String,
    pub sales_count: i64,
    pub amount: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayablesSummary {
    pub pending_total: i64,
    pub overdue_total: i64,
    pub due_soon_total: i64,
    pub active_count: i64,
    pub overdue_count: i64,
    pub due_soon_count: i64,
    pub paid_in_range: i64,
    pub payments_in_range_count: i64,
    pub issued_in_range: i64,
    pub by_category: Vec<PayablesByCategory>,
    pub by_urgency: Vec<UrgencyBucket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayablesByCategory {
    pub category: DebtCategory,
    pub count: i64,
    pub balance_total: i64,
    pub overdue_total: i64,
    pub share_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrgencyBucket {
    pub urgency: DebtUrgency,
    pub count: i64,
    pub balance_total: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgingLabel {
    #[serde(rename = "not_due")]
    NotDue,
    #[serde(rename = "overdue_1_30")]
    Overdue1to30,
    #[serde(rename = "overdue_31_60")]
    Overdue31to60,
    #[serde(rename = "overdue_61_90")]
    Overdue61to90,
    #[serde(rename = "overdue_90_plus")]
    Overdue90Plus,
    #[serde(rename = "no_due_date")]
    NoDueDate,
}

/// Balance actual por antigüedad del vencimiento de deudas activas con saldo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgingBucket {
    pub label: AgingLabel,
    pub count: i64,
    pub balance_total: i64,
}

/// Creditor names are grouped by exact string; spelling variations stay separate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopCreditor {
    pub creditor_name: String,
    pub active_count: i64,
    pub balance_total: i64,
    pub overdue_total: i64,
    pub next_due_on: Option<NaiveDate>,
}

/// The report filter's category_id is intentionally ignored for payables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpcomingDue {
    pub debt_id: i32,
    pub creditor_name: String,
    pub concept: String,
    pub category: DebtCategory,
    pub due_on: NaiveDate,
    pub balance: i64,
    pub days_until_due: i32,
    pub urgency: DebtUrgency,
}

#[cfg(test)]
mod tests {
    use super::{AgingLabel, Granularity, RankBy, ReportFilter};

    #[test]
    fn aging_labels_serialize_to_their_explicit_api_names() {
        let labels = [
            (AgingLabel::NotDue, "not_due"),
            (AgingLabel::Overdue1to30, "overdue_1_30"),
            (AgingLabel::Overdue31to60, "overdue_31_60"),
            (AgingLabel::Overdue61to90, "overdue_61_90"),
            (AgingLabel::Overdue90Plus, "overdue_90_plus"),
            (AgingLabel::NoDueDate, "no_due_date"),
        ];

        for (label, expected) in labels {
            assert_eq!(
                serde_json::to_string(&label).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn report_filter_defaults_to_daily_without_comparison() {
        let filter: ReportFilter =
            serde_json::from_str(r#"{"date_from":"2026-10-01","date_to":"2026-10-02"}"#)
                .expect("deserialize filter with defaults");

        assert_eq!(filter.granularity, Some(Granularity::Day));
        assert!(!filter.compare_previous);
        assert_eq!(filter.category_id, None);
        assert_eq!(filter.limit, None);
        assert_eq!(filter.rank_by, Some(RankBy::Revenue));
    }
}

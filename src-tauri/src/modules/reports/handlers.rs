use tauri::State;

use crate::{
    db::DbPool,
    errors::app_error::AppError,
    modules::reports::{
        dto::{
            AgingBucket, CategorySales, DashboardKpis, HeatmapCell, InventoryTurnover,
            InventoryValuation, PayablesSummary, PaymentMethodStat, ReportFilter, StaleProduct,
            StockAlert, TimeSeriesPoint, TopCreditor, TopProduct, UpcomingDue,
        },
        service,
    },
};

#[tauri::command]
pub async fn get_dashboard_kpis(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<DashboardKpis, String> {
    service::get_dashboard_kpis(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_sales_series(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<TimeSeriesPoint>, String> {
    service::get_sales_series(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_sales_by_payment_method(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<PaymentMethodStat>, String> {
    service::get_sales_by_payment_method(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_sales_heatmap(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<HeatmapCell>, String> {
    service::get_sales_heatmap(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_top_products(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<TopProduct>, String> {
    service::get_top_products(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_sales_by_category(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<CategorySales>, String> {
    service::get_sales_by_category(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_stale_products(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<StaleProduct>, String> {
    service::get_stale_products(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_inventory_valuation(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<InventoryValuation, String> {
    service::get_inventory_valuation(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_inventory_turnover(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<InventoryTurnover, String> {
    service::get_inventory_turnover(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_stock_alerts(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<StockAlert>, String> {
    service::get_stock_alerts(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_payables_summary(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<PayablesSummary, String> {
    service::get_payables_summary(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_payables_aging(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<AgingBucket>, String> {
    service::get_payables_aging(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_top_creditors(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<TopCreditor>, String> {
    service::get_top_creditors(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_upcoming_dues(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<UpcomingDue>, String> {
    service::get_upcoming_dues(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn save_report_csv(file_path: String, contents: String) -> Result<(), String> {
    tokio::fs::write(file_path, contents)
        .await
        .map_err(|error| format!("No se pudo guardar el archivo CSV: {error}"))
}

use tauri::State;

use crate::{
    db::DbPool,
    errors::app_error::AppError,
    modules::reports::{
        dto::{DashboardKpis, HeatmapCell, PaymentMethodStat, ReportFilter, TimeSeriesPoint},
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

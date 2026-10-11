use tauri::State;

use crate::{
    db::DbPool,
    errors::app_error::AppError,
    modules::reports::{
        dto::{DashboardKpis, ReportFilter},
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

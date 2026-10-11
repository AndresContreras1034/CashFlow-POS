use std::path::{Path, PathBuf};
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
    let file_path = validate_report_csv_path(&file_path, contents.len())?;
    tokio::fs::write(&file_path, contents)
        .await
        .map_err(|error| {
            format!(
                "No se pudo guardar el archivo CSV en {}: {error}",
                file_path.display()
            )
        })
}

const MAX_REPORT_CSV_BYTES: usize = 20 * 1024 * 1024;

fn validate_report_csv_path(file_path: &str, contents_len: usize) -> Result<PathBuf, String> {
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return Err("La ruta del CSV debe ser absoluta".to_string());
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
    {
        return Err("El archivo debe tener extensión .csv".to_string());
    }
    if contents_len > MAX_REPORT_CSV_BYTES {
        return Err("El archivo CSV no puede superar 20 MB".to_string());
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod report_csv_tests {
    use super::{save_report_csv, validate_report_csv_path, MAX_REPORT_CSV_BYTES};
    use std::{fs, path::PathBuf};

    fn temp_dir() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("cashflow-report-csv-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).expect("temporary test directory should be created");
        path
    }

    #[test]
    fn validates_csv_path_and_size() {
        let directory = temp_dir();
        let valid_path = directory.join("report.csv");
        let invalid_extension = directory.join("report.txt");

        assert!(validate_report_csv_path("relative.csv", 1)
            .unwrap_err()
            .contains("absoluta"));
        assert!(
            validate_report_csv_path(invalid_extension.to_str().unwrap(), 1)
                .unwrap_err()
                .contains(".csv")
        );
        assert!(
            validate_report_csv_path(valid_path.to_str().unwrap(), MAX_REPORT_CSV_BYTES + 1)
                .unwrap_err()
                .contains("20 MB")
        );
        assert!(
            validate_report_csv_path(valid_path.to_str().unwrap(), MAX_REPORT_CSV_BYTES).is_ok()
        );

        fs::remove_dir(directory).expect("temporary test directory should be removed");
    }

    #[tokio::test]
    async fn writes_csv_to_a_temporary_directory() {
        let directory = temp_dir();
        let path = directory.join("report.csv");
        let path_text = path.to_string_lossy().into_owned();

        save_report_csv(path_text, "a;b\r\n1;2".to_string())
            .await
            .expect("valid CSV should be saved");
        assert_eq!(
            fs::read_to_string(&path).expect("saved CSV should be readable"),
            "a;b\r\n1;2"
        );

        fs::remove_file(&path).expect("temporary CSV should be removed");
        fs::remove_dir(directory).expect("temporary test directory should be removed");
    }
}

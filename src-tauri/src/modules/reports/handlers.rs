use chrono::NaiveDate;
use serde::Serialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

use crate::{
    db::DbPool,
    errors::app_error::AppError,
    modules::{
        audit::{
            dto::NewAuditEvent,
            models::{AuditCategory, AuditModule, AuditOutcome},
            repository as audit_repo,
        },
        reports::{
            dto::{
                AgingBucket, CategorySales, CustomerSalesSplit, DashboardKpis, HeatmapCell,
                InventoryTurnover, InventoryValuation, PayablesSummary, PaymentMethodStat,
                ReportFilter, StaleProduct, StockAlert, TimeSeriesPoint, TopCreditor, TopCustomer,
                TopProduct, UpcomingDue,
            },
            insights::{
                ai_secret, context as insights_context, generate as insights_generate,
                provider::DeepSeek, service as insights_service, store as insights_store,
            },
            service,
        },
    },
};

#[derive(Debug, Serialize)]
pub struct AiSettingsView {
    pub enabled: bool,
    pub model: String,
    pub daily_token_cap: i32,
    pub has_key: bool,
    pub tokens_today: i64,
}

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
pub async fn get_top_customers(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<TopCustomer>, String> {
    service::get_top_customers(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_customer_sales_split(
    pool: State<'_, DbPool>,
    filter: ReportFilter,
) -> Result<Vec<CustomerSalesSplit>, String> {
    service::get_customer_sales_split(&pool, filter)
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
pub async fn get_commercial_context(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    let context = insights_service::build_context(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let fingerprint = insights_context::fingerprint(&context).map_err(|error| error.to_string())?;
    let mut payload = serde_json::to_value(context).map_err(|error| error.to_string())?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "El contexto comercial no es un objeto JSON".to_string())?;
    object.insert(
        "fingerprint".to_string(),
        serde_json::Value::String(fingerprint),
    );
    Ok(payload)
}

#[tauri::command]
pub async fn get_ai_advice(
    pool: State<'_, DbPool>,
    scope: String,
) -> Result<Option<insights_store::StoredAdvice>, String> {
    insights_store::latest_ok(&pool, &scope)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn generate_ai_advice(
    pool: State<'_, DbPool>,
    scope: String,
) -> Result<insights_store::StoredAdvice, String> {
    let context = insights_service::build_context(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let fingerprint = insights_context::fingerprint(&context).map_err(|error| error.to_string())?;
    let mut input = serde_json::to_value(context).map_err(|error| error.to_string())?;
    let input_object = input
        .as_object_mut()
        .ok_or_else(|| "El contexto comercial no es un objeto JSON".to_string())?;
    input_object.insert("fingerprint".to_string(), json!(fingerprint));

    let (enabled, model, _) = insights_store::load_settings(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let api_key = ai_secret::get_key()?.unwrap_or_default();
    if enabled && api_key.trim().is_empty() {
        return Err("Configura la clave de DeepSeek en Ajustes".to_string());
    }
    let provider = DeepSeek::new(api_key, model.clone()).map_err(|error| error.to_string())?;
    let advice = insights_generate::generate(&pool, &provider, &scope, &input)
        .await
        .map_err(|error| error.to_string())?;
    let (prompt_tokens, completion_tokens) = insights_store::token_usage(&pool, advice.id)
        .await
        .map_err(|error| error.to_string())?;

    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    audit_repo::insert_event(
        &mut tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::System,
            action: "ai_advice_generate".to_string(),
            outcome: AuditOutcome::Success,
            actor: None,
            entity_type: Some("ai_advice_daily".to_string()),
            entity_id: Some(advice.id.to_string()),
            summary: format!(
                "Consejo IA generado: modelo {model}; prompt_tokens={prompt_tokens}; completion_tokens={completion_tokens}"
            ),
            changes: None,
            metadata: Some(json!({
                "scope": scope,
                "model": model,
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
            })),
            error_message: None,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;

    Ok(advice)
}

#[tauri::command]
pub async fn get_ai_settings(pool: State<'_, DbPool>) -> Result<AiSettingsView, String> {
    let (enabled, model, daily_token_cap) = insights_store::load_settings(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let today: NaiveDate = sqlx::query_scalar(
        "SELECT (NOW() AT TIME ZONE timezone)::DATE FROM app_settings WHERE id = 1",
    )
    .fetch_one(&*pool)
    .await
    .map_err(|error| error.to_string())?;
    let state = insights_store::day_state(&pool, today, "dashboard")
        .await
        .map_err(|error| error.to_string())?;

    Ok(AiSettingsView {
        enabled,
        model,
        daily_token_cap,
        has_key: ai_secret::has_key()?,
        tokens_today: state.tokens_today,
    })
}

#[tauri::command]
pub async fn update_ai_settings(
    pool: State<'_, DbPool>,
    enabled: bool,
    daily_token_cap: i32,
    api_key: Option<String>,
) -> Result<AiSettingsView, String> {
    if !(1_000..=1_000_000).contains(&daily_token_cap) {
        return Err("El tope diario debe estar entre 1.000 y 1.000.000 tokens".to_string());
    }
    let api_key_updated = if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) {
        ai_secret::set_key(key.trim())?;
        true
    } else {
        false
    };

    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query("UPDATE ai_settings SET enabled = $1, daily_token_cap = $2 WHERE id = 1")
        .bind(enabled)
        .bind(daily_token_cap)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    audit_repo::insert_event(
        &mut tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Settings,
            action: "ai_settings_update".to_string(),
            outcome: AuditOutcome::Success,
            actor: None,
            entity_type: Some("ai_settings".to_string()),
            entity_id: Some("1".to_string()),
            summary: "Ajustes del asesor de IA actualizados".to_string(),
            changes: Some(json!({
                "enabled": enabled,
                "daily_token_cap": daily_token_cap,
                "api_key_updated": api_key_updated,
            })),
            metadata: None,
            error_message: None,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;

    get_ai_settings(pool).await
}

#[tauri::command]
pub async fn set_ai_feedback(
    pool: State<'_, DbPool>,
    id: i64,
    feedback: String,
) -> Result<(), String> {
    if feedback != "useful" && feedback != "not_applicable" {
        return Err("Feedback inválido".to_string());
    }
    insights_store::set_feedback(&pool, id, &feedback)
        .await
        .map_err(|error| error.to_string())
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

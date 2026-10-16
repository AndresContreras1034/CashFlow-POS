use tauri::State;

use crate::{
    db::DbPool,
    modules::commercial_calendar::{dto::CommercialEventInput, service},
};

#[tauri::command]
pub async fn list_commercial_events(
    pool: State<'_, DbPool>,
) -> Result<Vec<super::dto::CommercialEvent>, String> {
    service::list_events(&pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_commercial_event(
    pool: State<'_, DbPool>,
    dto: CommercialEventInput,
) -> Result<super::dto::CommercialEvent, String> {
    service::create_event(&pool, dto)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_commercial_event(
    pool: State<'_, DbPool>,
    id: i64,
    dto: CommercialEventInput,
) -> Result<super::dto::CommercialEvent, String> {
    service::update_event(&pool, id, dto)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_commercial_event(
    pool: State<'_, DbPool>,
    id: i64,
    actor: Option<String>,
) -> Result<(), String> {
    service::delete_event(&pool, id, actor)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_upcoming_commercial_events(
    pool: State<'_, DbPool>,
    horizon_days: i64,
) -> Result<Vec<super::dto::UpcomingCommercialEvent>, String> {
    service::upcoming_events(&pool, horizon_days)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_current_sales_pace(
    pool: State<'_, DbPool>,
    days: u16,
) -> Result<super::dto::SalesPace, String> {
    service::current_sales_pace(&pool, days)
        .await
        .map_err(|error| error.to_string())
}

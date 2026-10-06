use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::settings::{dto::UpdateSettingsDto, models::AppSettings, service};

#[tauri::command]
pub async fn get_settings(pool: State<'_, DbPool>) -> Result<AppSettings, String> {
    service::get_settings(&pool)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    pool: State<'_, DbPool>,
    dto: UpdateSettingsDto,
) -> Result<AppSettings, String> {
    service::update_settings(&pool, dto)
        .await
        .map_err(|e: AppError| e.to_string())
}

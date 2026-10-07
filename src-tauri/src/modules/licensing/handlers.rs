use tauri::State;

use crate::{
    db::DbPool,
    modules::licensing::{models::LicenseStatusDto, service},
};

#[tauri::command]
pub async fn get_license_status(pool: State<'_, DbPool>) -> Result<LicenseStatusDto, String> {
    service::get_status(&pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn activate_license(pool: State<'_, DbPool>, file_path: String) -> Result<(), String> {
    service::activate(&pool, &file_path)
        .await
        .map_err(|error| error.to_string())
}

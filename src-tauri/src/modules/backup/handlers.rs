use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::backup::{
    dto::{
        BackupEnvironment, CreateRecipientDto, EmailTestResult, SecretsStatus,
        UpdateBackupSettingsDto, UpdateRecipientDto,
    },
    models::{BackupDelivery, BackupKind, BackupRecipient, BackupRecord, BackupSettings},
    notify, service,
    status::{self, BackupStatus},
};

pub fn default_backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .document_dir()
        .map(|dir| dir.join("CashFlow-Backups"))
        .map_err(|e| format!("No se pudo resolver la carpeta Documentos: {e}"))
}

#[tauri::command]
pub async fn get_backup_status(pool: State<'_, DbPool>) -> Result<BackupStatus, String> {
    status::get(&pool)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn get_backup_settings(pool: State<'_, DbPool>) -> Result<BackupSettings, String> {
    service::get_settings(&pool)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn update_backup_settings(
    pool: State<'_, DbPool>,
    dto: UpdateBackupSettingsDto,
) -> Result<BackupSettings, String> {
    service::update_settings(&pool, dto)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn check_backup_environment(
    app: AppHandle,
    pool: State<'_, DbPool>,
) -> Result<BackupEnvironment, String> {
    let default_dir = default_backup_dir(&app)?;
    service::check_environment(&pool, &default_dir)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn run_backup_now(
    app: AppHandle,
    pool: State<'_, DbPool>,
    actor: Option<String>,
) -> Result<BackupRecord, String> {
    let default_dir = default_backup_dir(&app)?;
    service::run_backup(&pool, &default_dir, BackupKind::Manual, actor)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_backup_history(
    pool: State<'_, DbPool>,
    limit: Option<i64>,
) -> Result<Vec<BackupRecord>, String> {
    service::list_history(&pool, limit)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub fn is_backup_running() -> bool {
    service::is_running()
}

#[tauri::command]
pub fn get_backup_secrets_status() -> Result<SecretsStatus, String> {
    service::secrets_status().map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn set_backup_encryption_password(
    pool: State<'_, DbPool>,
    password: Option<String>,
) -> Result<SecretsStatus, String> {
    service::set_encryption_password(&pool, password)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn set_backup_smtp_password(
    pool: State<'_, DbPool>,
    password: Option<String>,
) -> Result<SecretsStatus, String> {
    service::set_smtp_password(&pool, password)
        .await
        .map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn list_backup_recipients(pool: State<'_, DbPool>) -> Result<Vec<BackupRecipient>, String> {
    service::list_recipients(&pool).await.map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn add_backup_recipient(
    pool: State<'_, DbPool>,
    dto: CreateRecipientDto,
) -> Result<BackupRecipient, String> {
    service::add_recipient(&pool, dto).await.map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn update_backup_recipient(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateRecipientDto,
) -> Result<BackupRecipient, String> {
    service::update_recipient(&pool, id, dto).await.map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn delete_backup_recipient(pool: State<'_, DbPool>, id: i32) -> Result<(), String> {
    service::delete_recipient(&pool, id).await.map_err(|e: AppError| e.to_string())
}

#[tauri::command]
pub async fn send_backup_test_email(pool: State<'_, DbPool>) -> Result<Vec<EmailTestResult>, String> {
    notify::send_test(&pool).await
}

#[tauri::command]
pub async fn list_backup_deliveries(
    pool: State<'_, DbPool>,
    limit: Option<i64>,
) -> Result<Vec<BackupDelivery>, String> {
    service::list_deliveries(&pool, limit).await.map_err(|e: AppError| e.to_string())
}

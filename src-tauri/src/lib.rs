pub mod config;
pub mod db;
pub mod errors;
pub mod middleware;
pub mod modules;
pub mod router;
pub mod types;

use db::init_db;
use tauri::Manager;
use modules::inventory::router::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pos=debug,sqlx=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let pool = init_db()
                    .await
                    .expect("Error al inicializar la base de datos");

                handle.manage(pool);
                tracing::info!("Base de datos lista.");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_categories,
            get_category,
            create_category,
            update_category,
            list_products,
            get_product,
            create_product,
            update_product,
            deactivate_product,
            list_variants,
            get_variant,
            find_by_barcode,
            search_variants,
            create_variant,
            update_variant,
            register_stock_entry,
            register_initial_stock,
            register_manual_entry,
            register_manual_out,
            adjust_stock,
            get_kardex,
            get_low_stock,
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri");
}
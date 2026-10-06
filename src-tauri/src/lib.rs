pub mod config;
pub mod db;
pub mod errors;
pub mod middleware;
pub mod modules;
pub mod router;
pub mod types;

use db::init_db;
use modules::billing::router::*;
use modules::cash::router::*;
use modules::inventory::router::*;
use modules::sales::router::*;
use modules::settings::router::*;
use modules::stocktake::router::*;
use tauri::Manager;

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
        // =====================================================
        // Plugins
        // =====================================================
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // =====================================================
        // Inicialización
        // =====================================================
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let pool = init_db()
                    .await
                    .expect("Error al inicializar la base de datos");

                handle.manage(pool);

                tracing::info!("Base de datos lista.");

                if let Some(window) = handle.get_webview_window("main") {
                    window.show().ok();
                }
            });

            Ok(())
        })
        // =====================================================
        // Comandos Tauri
        // =====================================================
        .invoke_handler(tauri::generate_handler![
            // =================================================
            // Categories
            // =================================================
            list_categories,
            get_category,
            create_category,
            update_category,
            // =================================================
            // Products
            // =================================================
            list_products,
            get_product,
            create_product,
            update_product,
            deactivate_product,
            // =================================================
            // Variants
            // =================================================
            list_variants,
            get_variant,
            find_by_barcode,
            generate_internal_barcode,
            search_variants,
            create_variant,
            update_variant,
            // =================================================
            // Inventory
            // =================================================
            register_stock_entry,
            register_initial_stock,
            register_manual_entry,
            register_manual_out,
            adjust_stock,
            get_kardex,
            get_low_stock,
            get_inventory_value,
            get_product_stock_stats,
            // =================================================
            // Importación masiva de inventario
            // =================================================
            preview_import_inventory,
            execute_import_inventory,
            export_inventory,
            export_inventory_template,
            print_variant_labels,
            // =================================================
            // Stocktake
            // =================================================
            start_stocktake,
            get_current_stocktake,
            list_stocktakes,
            list_stocktake_lines,
            find_stocktake_line_by_code,
            set_stocktake_count,
            get_stocktake_review,
            apply_stocktake,
            cancel_stocktake,
            // =================================================
            // Settings
            // =================================================
            get_settings,
            update_settings,
            // =================================================
            // Cash
            // =================================================
            open_cash_session,
            close_cash_session,
            get_current_cash_session,
            get_cash_session,
            list_cash_sessions,
            register_cash_movement,
            list_cash_movements,
            // =================================================
            // Sales
            // =================================================
            create_sale,
            get_sale,
            list_sales,
            // =================================================
            // Billing / Printer
            // =================================================
            print_sale_ticket,
        ])
        // =====================================================
        // Ejecutar aplicación
        // =====================================================
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri");
}

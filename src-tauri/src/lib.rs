pub mod config;
pub mod db;
pub mod errors;
pub mod logging;
pub mod middleware;
pub mod modules;
pub mod router;
pub mod types;

use db::connection::probe_health;
use db::init_db;
use modules::audit::router::*;
use modules::billing::router::*;
use modules::cash::router::*;
use modules::developer::router::*;
use modules::inventory::router::*;
use modules::licensing::router::*;
use modules::sales::router::*;
use modules::settings::router::*;
use modules::stocktake::router::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = std::time::Instant::now();
    dotenvy::dotenv().ok();

    let developer_state = modules::developer::state::shared();
    logging::init(developer_state.clone());

    tauri::Builder::default()
        // =====================================================
        // Plugins
        // =====================================================
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // =====================================================
        // Inicialización
        // =====================================================
        .manage(developer_state)
        .setup(move |app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match init_db().await {
                    Ok(pool) => {
                        let health_pool = pool.clone();
                        handle.manage(pool);

                        tracing::info!(ok = true, "Base de datos lista");
                        let startup = started.elapsed();

                        if let Some(window) = handle.get_webview_window("main") {
                            window.show().ok();
                        }

                        let db = probe_health(&health_pool)
                            .await
                            .map_err(|error| error.to_string());
                        logging::print_startup_panel(&logging::StartupInfo {
                            app_version: env!("CARGO_PKG_VERSION"),
                            profile: if cfg!(debug_assertions) {
                                "debug"
                            } else {
                                "release"
                            },
                            log_filter: logging::effective_filter(
                                std::env::var("RUST_LOG").ok().as_deref(),
                            ),
                            db,
                            pool_size: health_pool.size(),
                            pool_idle: health_pool.num_idle(),
                            pool_max: health_pool.options().get_max_connections(),
                            startup,
                        });
                    }
                    Err(error) => {
                        tracing::error!(
                            error = %error,
                            "No se pudo inicializar la base de datos"
                        );
                        handle.exit(1);
                    }
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
            // Licencias offline
            // =================================================
            get_license_status,
            activate_license,
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
            // =================================================
            // Audit
            // =================================================
            list_audit_events,
            get_audit_event,
            list_audit_events_by_correlation,
            // Developer Mode
            set_developer_mode,
            get_developer_status,
            get_developer_events,
            clear_developer_events,
            get_health,
        ])
        // =====================================================
        // Ejecutar aplicación
        // =====================================================
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri");
}

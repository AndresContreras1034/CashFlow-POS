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
use modules::backup::router::*;
use modules::billing::router::*;
use modules::cash::router::*;
use modules::customers::router::*;
use modules::debts::router::*;
use modules::developer::router::*;
use modules::inventory::router::*;
use modules::licensing::router::*;
use modules::reports::router::*;
use modules::sales::router::*;
use modules::settings::router::*;
use modules::stocktake::router::*;
use modules::suppliers::router::*;
use tauri::Manager;

#[cfg(windows)]
fn show_fatal_dialog(title: &str, message: &str) {
    use std::ptr::null_mut;
    use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};
    let to_wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let text = to_wide(message);
    let cap = to_wide(title);
    unsafe {
        MessageBoxW(null_mut(), text.as_ptr(), cap.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn show_fatal_dialog(title: &str, message: &str) {
    eprintln!("{title}: {message}");
}

fn load_runtime_environment(app: &tauri::AppHandle) -> Result<(), String> {
    if std::env::var_os("DATABASE_URL").is_some() {
        return Ok(());
    }

    let config_file = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("No se pudo resolver la configuración de la aplicación: {error}"))?
        .join(".env");

    if config_file.exists() {
        dotenvy::from_path(&config_file).map_err(|error| {
            format!("No se pudo leer la configuración de la aplicación: {error}")
        })?;
        return Ok(());
    }

    #[cfg(debug_assertions)]
    {
        let development_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
        if development_file.exists() {
            dotenvy::from_path(&development_file).map_err(|error| {
                format!("No se pudo leer la configuración de desarrollo: {error}")
            })?;
            return Ok(());
        }
    }

    Err(format!(
        "No se encontró DATABASE_URL. Configura {}",
        config_file.display()
    ))
}

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

            if let Err(error) = load_runtime_environment(&handle) {
                tracing::error!("{error}");
                show_fatal_dialog(
                    "Error de Configuración - CashFlow POS",
                    &format!(
                        "No se pudo cargar la configuración de la aplicación (DATABASE_URL).\n\n\
                         Verifica que exista el archivo .env con la conexión a PostgreSQL o configura las variables de entorno del sistema.\n\n\
                         Detalle: {error}"
                    ),
                );
                handle.exit(1);
                return Ok(());
            }

            let pool = match tauri::async_runtime::block_on(init_db()) {
                Ok(pool) => pool,
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        "No se pudo inicializar la base de datos"
                    );
                    show_fatal_dialog(
                        "Error de Base de Datos - CashFlow POS",
                        &format!(
                            "No se pudo conectar a la base de datos PostgreSQL.\n\n\
                             Verifica que el servicio de PostgreSQL esté iniciado y accesible en este equipo, y que las credenciales en .env sean correctas.\n\n\
                             Detalle: {error}"
                        ),
                    );
                    handle.exit(1);
                    return Ok(());
                }
            };
            let health_pool = pool.clone();
            let scheduler_pool = pool.clone();
            if !handle.manage(pool) {
                tracing::error!("No se pudo registrar el pool de base de datos");
                show_fatal_dialog(
                    "Error de Inicio - CashFlow POS",
                    "No se pudo registrar el pool de base de datos en la aplicación.",
                );
                handle.exit(1);
                return Ok(());
            }

            if let Some(window) = handle.get_webview_window("main") {
                window.show().ok();
            }

            modules::backup::scheduler::spawn(handle.clone(), scheduler_pool);

            tauri::async_runtime::spawn(async move {
                tracing::info!(ok = true, "Base de datos lista");
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
                    startup: started.elapsed(),
                });
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
            get_cash_session_report,
            list_cash_sessions,
            register_cash_movement,
            list_cash_movements,
            print_cash_report,
            // =================================================
            // Sales
            // =================================================
            create_sale,
            get_sale,
            list_sales,
            void_sale,
            assign_sale_customer,
            // =================================================
            // Billing / Printer
            // =================================================
            print_sale_ticket,
            // =================================================
            // Reports
            // =================================================
            get_dashboard_kpis,
            get_sales_series,
            get_sales_by_payment_method,
            get_sales_heatmap,
            get_top_products,
            get_sales_by_category,
            get_stale_products,
            get_inventory_valuation,
            get_inventory_turnover,
            get_stock_alerts,
            get_payables_summary,
            get_payables_aging,
            get_top_creditors,
            get_upcoming_dues,
            get_top_customers,
            get_customer_sales_split,
            save_report_csv,
            // =================================================
            // Audit
            // =================================================
            list_audit_events,
            get_audit_event,
            list_audit_events_by_correlation,
            // =================================================
            // Debts
            // =================================================
            get_debt_options,
            list_debts,
            get_debt,
            get_debt_summary,
            get_debt_category_stats,
            list_due_debts,
            list_debt_history,
            create_debt,
            update_debt,
            void_debt,
            register_debt_payment,
            void_debt_payment,
            // =================================================
            // Customers
            // =================================================
            list_customers,
            get_customer,
            get_customer_names,
            create_customer,
            update_customer,
            set_customer_active,
            // =================================================
            // Suppliers
            // =================================================
            list_suppliers,
            get_supplier,
            create_supplier,
            update_supplier,
            set_supplier_active,
            set_supplier_categories,
            list_supplier_categories,
            link_supplier_variant,
            update_supplier_variant,
            unlink_supplier_variant,
            list_supplier_variants,
            list_variant_suppliers,
            // =================================================
            // Backup
            // =================================================
            get_backup_settings,
            get_backup_status,
            update_backup_settings,
            check_backup_environment,
            run_backup_now,
            list_backup_history,
            is_backup_running,
            get_backup_secrets_status,
            set_backup_encryption_password,
            set_backup_smtp_password,
            list_backup_recipients,
            add_backup_recipient,
            update_backup_recipient,
            delete_backup_recipient,
            send_backup_test_email,
            list_backup_deliveries,
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

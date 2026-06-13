/// Retorna todos los comandos del módulo de inventario
/// para ser registrados en tauri::generate_handler!
///
/// Uso en lib.rs:
///   .invoke_handler(tauri::generate_handler![
///       ...inventory::router::commands(),  // no funciona así en Tauri
///   ])
///
/// En Tauri los comandos se deben listar explícitamente en generate_handler!
/// Este archivo sirve como referencia de todos los comandos disponibles.
/// Ver lib.rs para el registro final.

pub use super::handlers::{
    // Categorías
    list_categories,
    get_category,
    create_category,
    update_category,

    // Productos
    list_products,
    get_product,
    create_product,
    update_product,
    deactivate_product,

    // Variantes
    list_variants,
    get_variant,
    find_by_barcode,
    search_variants,
    create_variant,
    update_variant,

    // Stock
    register_stock_entry,
    register_initial_stock,
    register_manual_entry,
    register_manual_out,
    adjust_stock,

    // Kardex
    get_kardex,

    // Alertas
    get_low_stock,
};
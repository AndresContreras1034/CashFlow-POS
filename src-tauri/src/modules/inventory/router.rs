/// Retorna todos los comandos del módulo de inventario
///
/// para ser registrados en tauri::generate_handler!
///
/// En Tauri los comandos se deben listar explícitamente en generate_handler!
/// Este archivo sirve como referencia de todos los comandos disponibles.
/// Ver lib.rs para el registro final.
pub use super::handlers::{
    adjust_stock,

    create_category,
    create_product,
    create_variant,
    deactivate_product,

    execute_import_inventory,
    export_inventory,
    export_inventory_template,

    find_by_barcode,
    generate_internal_barcode,
    get_category,
    get_inventory_value,
    // Kardex
    get_kardex,

    // Alertas
    get_low_stock,
    get_product,
    get_product_stock_stats,
    get_variant,
    // Categorías
    list_categories,
    // Productos
    list_products,
    // Variantes
    list_variants,
    // Importación masiva
    preview_import_inventory,
    print_variant_labels,
    register_initial_stock,
    register_manual_entry,
    register_manual_out,
    // Stock
    register_stock_entry,
    search_variants,
    update_category,

    update_product,
    update_variant,
};

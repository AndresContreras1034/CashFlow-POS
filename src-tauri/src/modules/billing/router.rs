// router.rs
//
// Re-exporta los comandos de Billing para que puedan
// registrarse desde lib.rs en tauri::generate_handler![...].

pub use crate::modules::billing::handlers::print_sale_ticket;

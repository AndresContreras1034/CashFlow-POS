use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::modules::sales::models::{PaymentMethod, SaleStatus};

/// Una línea de producto en el ticket, ya con el nombre resuelto
/// (SaleItem solo trae variant_id — para el ticket necesitamos texto legible).
#[derive(Debug, Clone, Serialize)]
pub struct TicketLine {
    pub product_name: String,
    pub attributes: String, // "Talla: M, Color: Azul" ya formateado para imprimir
    pub sku: Option<String>,
    pub quantity: i32,
    pub unit_price: i64,
    pub discount: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TicketPayment {
    pub method: PaymentMethod,
    pub amount: i64,
}

/// Todo lo que necesita ticket.rs para armar el recibo.
/// Se ensambla en el handler a partir de sales + inventory + settings.
#[derive(Debug, Clone, Serialize)]
pub struct TicketData {
    // Datos del negocio (vienen de app_settings)
    pub business_name: String,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub ticket_header: Option<String>,
    pub ticket_footer: Option<String>,

    // Print settings (from app_settings)
    pub tax_name: String,
    pub currency_decimals: u8,
    pub show_logo: bool,
    pub show_tax_id: bool,
    pub show_address: bool,
    pub show_phone: bool,
    pub show_cashier: bool,
    pub show_tax_breakdown: bool,
    pub show_discounts: bool,
    pub show_payment_method: bool,

    // Datos de la venta
    pub sale_id: i32,
    pub ticket_number: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub status: SaleStatus,

    pub lines: Vec<TicketLine>,
    pub payments: Vec<TicketPayment>,

    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub total: i64,
}

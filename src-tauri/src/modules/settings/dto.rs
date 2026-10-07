use serde::Deserialize;

// No existe CreateSettingsDto: la fila única se siembra en la migración.
// Solo se puede leer y actualizar.

// Semántica de actualización:
// - Los campos NOT NULL: None = no cambiar.
// - Los campos nullable se reemplazan siempre; None = borrar (NULL).
//   El frontend debe enviar el objeto completo.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsDto {
    pub business_name: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub currency: Option<String>,
    pub currency_decimals: Option<i16>,
    pub tax_rate_bps: Option<i32>,
    pub tax_name: Option<String>,
    pub timezone: Option<String>,
    pub ticket_header: Option<String>,
    pub ticket_footer: Option<String>,
    pub show_logo: Option<bool>,
    pub show_tax_id: Option<bool>,
    pub show_address: Option<bool>,
    pub show_phone: Option<bool>,
    pub show_cashier: Option<bool>,
    pub show_tax_breakdown: Option<bool>,
    pub show_discounts: Option<bool>,
    pub show_payment_method: Option<bool>,
    pub low_stock_default_threshold: Option<i32>,
    pub logo_url: Option<String>,
}

use serde::Deserialize;

// No existe CreateSettingsDto: la fila única se siembra en la migración.
// Solo se puede leer y actualizar.

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsDto {
    pub business_name: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub currency: Option<String>,
    pub tax_rate_bps: Option<i32>,
    pub ticket_header: Option<String>,
    pub ticket_footer: Option<String>,
    pub low_stock_default_threshold: Option<i32>,
    pub logo_url: Option<String>,
}

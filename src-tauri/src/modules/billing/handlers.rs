use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tauri::State;

use crate::modules::billing::{
    counter,
    models::{TicketData, TicketLine, TicketPayment},
    printer, ticket,
};
use crate::modules::sales::models::{PaymentMethod, SaleStatus};
use crate::modules::settings::repository as settings_repo;

// ------------------------------------------------------------
// Filas intermedias solo para esta consulta.
// ------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct SaleRow {
    id: i32,
    subtotal: i64,
    tax: i64,
    discount: i64,
    total: i64,
    status: SaleStatus,
    courtesy_reason: Option<String>,
    cash_received: Option<i64>,
    change_given: Option<i64>,
    created_by: String,
    created_at: DateTime<Utc>,
}

struct ItemRow {
    quantity: i32,
    unit_price: i64,
    discount: i64,
    subtotal: i64,
    product_name: String,
    sku: Option<String>,
    attributes: serde_json::Value,
}

struct PaymentRow {
    method: PaymentMethod,
    amount: i64,
}

// ------------------------------------------------------------
// Construcción de los datos del ticket
// ------------------------------------------------------------

async fn fetch_ticket_data(pool: &PgPool, sale_id: i32) -> Result<TicketData, String> {
    let sale = sqlx::query_as::<_, SaleRow>(
        r#"SELECT
               id,
               subtotal,
               tax,
               discount,
               total,
               status,
               courtesy_reason,
               cash_received,
               change_given,
               created_by,
               created_at
           FROM sales
           WHERE id = $1"#,
    )
    .bind(sale_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Error consultando la venta: {e}"))?
    .ok_or_else(|| format!("Venta #{sale_id} no encontrada"))?;

    let item_rows = sqlx::query_as!(
        ItemRow,
        r#"SELECT
               si.quantity,
               si.unit_price,
               si.discount,
               si.subtotal,
               p.name AS product_name,
               v.sku,
               v.attributes as "attributes!: serde_json::Value"
           FROM sale_items si
           JOIN product_variants v ON v.id = si.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE si.sale_id = $1
           ORDER BY si.id ASC"#,
        sale_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error consultando los items de la venta: {e}"))?;

    let payment_rows = sqlx::query_as!(
        PaymentRow,
        r#"SELECT
               method as "method: PaymentMethod",
               amount
           FROM sale_payments
           WHERE sale_id = $1
           ORDER BY id ASC"#,
        sale_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error consultando los pagos de la venta: {e}"))?;

    let settings = settings_repo::get_settings(pool)
        .await
        .map_err(|e| format!("Error consultando la configuracion del negocio: {e}"))?
        .ok_or_else(|| {
            "No hay configuracion del negocio (app_settings) registrada. \
             Ve a Ajustes y guarda el nombre del negocio antes de imprimir."
                .to_string()
        })?;

    let lines: Vec<TicketLine> = item_rows
        .into_iter()
        .map(|r| TicketLine {
            product_name: r.product_name,
            attributes: format_attributes(&r.attributes),
            sku: r.sku,
            quantity: r.quantity,
            unit_price: r.unit_price,
            discount: r.discount,
            subtotal: r.subtotal,
        })
        .collect();

    let payments: Vec<TicketPayment> = payment_rows
        .into_iter()
        .map(|r| TicketPayment {
            method: r.method,
            amount: r.amount,
        })
        .collect();

    Ok(TicketData {
        business_name: settings.business_name,
        tax_id: settings.tax_id,
        address: settings.address,
        phone: settings.phone,
        ticket_header: settings.ticket_header,
        ticket_footer: settings.ticket_footer,
        tax_name: settings.tax_name,
        currency_decimals: settings.currency_decimals as u8,
        show_logo: settings.show_logo,
        show_tax_id: settings.show_tax_id,
        show_address: settings.show_address,
        show_phone: settings.show_phone,
        show_cashier: settings.show_cashier,
        show_tax_breakdown: settings.show_tax_breakdown,
        show_discounts: settings.show_discounts,
        show_payment_method: settings.show_payment_method,

        sale_id: sale.id,
        ticket_number: counter::ticket_number(sale.id),
        created_at: sale.created_at,
        created_by: sale.created_by,
        status: sale.status,
        courtesy_reason: sale.courtesy_reason,

        lines,
        payments,

        subtotal: sale.subtotal,
        discount: sale.discount,
        tax: sale.tax,
        total: sale.total,
        cash_received: sale.cash_received,
        change_given: sale.change_given,
    })
}

// ------------------------------------------------------------
// Formatear atributos de una variante
// Ejemplo:
// {"talla": "M", "color": "Azul"}
// ->
// "talla: M, color: Azul"
// ------------------------------------------------------------

fn format_attributes(value: &serde_json::Value) -> String {
    match value.as_object() {
        Some(map) if !map.is_empty() => map
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.as_str().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join(", "),

        _ => String::new(),
    }
}

// ------------------------------------------------------------
// Comando Tauri:
// Imprime el ticket de una venta ya existente.
// ------------------------------------------------------------

#[tauri::command]
pub async fn print_sale_ticket(pool: State<'_, PgPool>, sale_id: i32) -> Result<(), String> {
    let data = fetch_ticket_data(pool.inner(), sale_id).await?;

    let bytes = ticket::build_sale_ticket(&data);

    printer::print_raw(&bytes)
}

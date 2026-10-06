//! Construcción del ticket en comandos ESC/POS, para papel térmico de 58mm
//! (impresora DIG-58IIA, 32 caracteres por línea aprox.).

use crate::modules::billing::models::{TicketData, TicketLine};

const CHARS_PER_LINE: usize = 32;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

// Logos pre-procesados a blanco/negro puro y ya convertidos a comandos
// ESC/POS raster (GS v 0) — ver /mnt/skills o el script de conversión que
// se usó para generarlos si algún día hay que regenerarlos con otro logo.
const LOGO_HEADER: &[u8] = include_bytes!("../../../assets/logo_header.bin");
const LOGO_FOOTER: &[u8] = include_bytes!("../../../assets/logo_footer.bin");

pub fn build_sale_ticket(data: &TicketData) -> Vec<u8> {
    let mut b = Vec::new();

    b.extend_from_slice(&[ESC, b'@']); // reset

    // ---- Logo (arriba) ----
    print_logo(&mut b, LOGO_HEADER);

    // ---- Encabezado: negocio + slogan ----
    align_center(&mut b);
    bold_on(&mut b);
    double_size_on(&mut b);
    push_line(&mut b, &ascii_safe(&data.business_name));
    double_size_off(&mut b);
    bold_off(&mut b);

    if let Some(slogan) = &data.ticket_header {
        push_line(&mut b, &ascii_safe(slogan));
    }

    if let Some(tax_id) = &data.tax_id {
        push_line(&mut b, &format!("NIT: {}", ascii_safe(tax_id)));
    }
    if let Some(address) = &data.address {
        for line in wrap(&ascii_safe(address), CHARS_PER_LINE) {
            push_line(&mut b, &line);
        }
    }
    if let Some(phone) = &data.phone {
        push_line(&mut b, &format!("Tel: {}", ascii_safe(phone)));
    }
    push_line(&mut b, "Servicio a domicilio");

    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));
    bold_on(&mut b);
    push_line(&mut b, "COMPROBANTE DE VENTA");
    bold_off(&mut b);
    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Datos de la venta ----
    align_left(&mut b);
    push_line(&mut b, &format!("No. de venta: {}", data.ticket_number));
    push_line(
        &mut b,
        &format!("Fecha: {}", data.created_at.format("%d/%m/%Y")),
    );
    push_line(
        &mut b,
        &format!("Hora: {}", data.created_at.format("%I:%M %p")),
    );
    push_line(
        &mut b,
        &format!("Vendedor: {}", ascii_safe(&data.created_by)),
    );

    // Cliente y Documento: SIN conectar a ningún dato real, se llenan a mano.
    push_line(&mut b, "Cliente: _______________________");
    push_line(&mut b, "Documento: _____________________");

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Productos ----
    for line in &data.lines {
        push_line(&mut b, &format_item_line(line));
    }

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Totales ----
    push_line(&mut b, &two_col("Subtotal:", &format_money(data.subtotal)));
    if data.discount > 0 {
        push_line(
            &mut b,
            &two_col("Descuento:", &format!("-{}", format_money(data.discount))),
        );
    }
    push_line(&mut b, &two_col("Impuestos:", &format_money(data.tax)));

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));
    bold_on(&mut b);
    push_line(&mut b, &two_col("TOTAL:", &format_money(data.total)));
    bold_off(&mut b);
    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Forma de pago ----
    align_center(&mut b);
    push_line(&mut b, "FORMA DE PAGO");
    align_left(&mut b);

    let cash: i64 = sum_by_method(data, crate::modules::sales::models::PaymentMethod::Cash);
    let transfer: i64 = sum_by_method(data, crate::modules::sales::models::PaymentMethod::Transfer);
    let card: i64 = sum_by_method(data, crate::modules::sales::models::PaymentMethod::Card);
    let total_received = cash + transfer + card;
    let change = (total_received - data.total).max(0);

    push_line(&mut b, &two_col("Efectivo:", &format_money(cash)));
    push_line(&mut b, &two_col("Transferencia:", &format_money(transfer)));
    push_line(&mut b, &two_col("Tarjeta:", &format_money(card)));
    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));
    push_line(
        &mut b,
        &two_col("Total recibido:", &format_money(total_received)),
    );
    push_line(&mut b, &two_col("Cambio:", &format_money(change)));

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Pie ----
    align_center(&mut b);
    bold_on(&mut b);
    push_line(&mut b, "GRACIAS POR TU COMPRA");
    bold_off(&mut b);
    push_line(&mut b, &ascii_safe(&data.business_name));
    if let Some(slogan) = &data.ticket_header {
        push_line(&mut b, &ascii_safe(slogan));
    }
    if let Some(phone) = &data.phone {
        push_line(&mut b, &format!("Tel: {}", ascii_safe(phone)));
    }
    push_line(&mut b, "Servicio a domicilio");
    if let Some(footer) = &data.ticket_footer {
        for line in wrap(&ascii_safe(footer), CHARS_PER_LINE) {
            push_line(&mut b, &line);
        }
    }

    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));

    // ---- Logo (abajo) ----
    //print_logo(&mut b, LOGO_FOOTER);

    b.extend_from_slice(b"\n\n\n");
    b.extend_from_slice(&[GS, b'V', 0x00]); // corte total
    b
}

// ============================================================
// Helpers
// ============================================================

fn sum_by_method(data: &TicketData, method: crate::modules::sales::models::PaymentMethod) -> i64 {
    data.payments
        .iter()
        .filter(|p| p.method == method)
        .map(|p| p.amount)
        .sum()
}

fn format_item_line(item: &TicketLine) -> String {
    let name = ascii_safe(&item.product_name);
    let mut out = String::new();

    for (i, wrapped) in wrap(&name, CHARS_PER_LINE).into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&wrapped);
    }

    if !item.attributes.is_empty() {
        out.push('\n');
        out.push_str(&ascii_safe(&item.attributes));
    }

    out.push('\n');
    let detail = format!("{} x {}", item.quantity, format_money(item.unit_price));
    out.push_str(&two_col(&detail, &format_money(item.subtotal)));

    out
}

fn two_col(left: &str, right: &str) -> String {
    let max_left = CHARS_PER_LINE.saturating_sub(right.len() + 1);
    let left_trunc: String = left.chars().take(max_left).collect();
    let padding = CHARS_PER_LINE
        .saturating_sub(left_trunc.len())
        .saturating_sub(right.len());
    format!("{}{}{}", left_trunc, " ".repeat(padding.max(1)), right)
}

fn format_money(cents: i64) -> String {
    let pesos = cents / 100;
    let negative = pesos < 0;
    let abs = pesos.abs().to_string();

    let mut grouped = String::new();
    for (i, c) in abs.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let grouped: String = grouped.chars().rev().collect();

    if negative {
        format!("-$ {}", grouped)
    } else {
        format!("$ {}", grouped)
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn ascii_safe(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' => 'O',
            'Ú' => 'U',
            'ñ' => 'n',
            'Ñ' => 'N',
            other => other,
        })
        .collect()
}

fn push_line(buf: &mut Vec<u8>, text: &str) {
    buf.extend_from_slice(text.as_bytes());
    buf.push(b'\n');
}

fn print_logo(buf: &mut Vec<u8>, logo_raster: &[u8]) {
    align_center(buf);
    buf.extend_from_slice(logo_raster);
    buf.push(b'\n');
}

fn align_center(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[ESC, b'a', 0x01]);
}

fn align_left(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[ESC, b'a', 0x00]);
}

fn bold_on(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[ESC, b'E', 0x01]);
}

fn bold_off(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[ESC, b'E', 0x00]);
}

fn double_size_on(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[GS, b'!', 0x11]); // doble ancho + doble alto
}

fn double_size_off(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[GS, b'!', 0x00]);
}

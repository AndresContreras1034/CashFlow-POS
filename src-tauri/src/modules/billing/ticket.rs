//! Construcción del ticket en comandos ESC/POS, para papel térmico de 58mm
//! (impresora DIG-58IIA, 32 caracteres por línea aprox.).

use crate::modules::billing::models::{CashReportTicketData, TicketData, TicketLine};
use crate::modules::sales::models::SaleStatus;

const CHARS_PER_LINE: usize = 32;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

// Logos pre-procesados a blanco/negro puro y ya convertidos a comandos
// ESC/POS raster (GS v 0) — ver /mnt/skills o el script de conversión que
// se usó para generarlos si algún día hay que regenerarlos con otro logo.
const LOGO_HEADER: &[u8] = include_bytes!("../../../assets/logo_header.bin");

pub fn build_sale_ticket(data: &TicketData) -> Vec<u8> {
    build_sale_ticket_with_copy(data, false)
}

pub fn build_sale_ticket_with_copy(data: &TicketData, copy: bool) -> Vec<u8> {
    let mut b = Vec::new();
    let money = |value: i64| format_money(value, data.currency_decimals);

    b.extend_from_slice(&[ESC, b'@']); // reset

    // ---- Logo (arriba) ----
    if data.show_logo {
        print_logo(&mut b, LOGO_HEADER);
    }

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

    if data.show_tax_id {
        if let Some(tax_id) = &data.tax_id {
            push_line(&mut b, &format!("NIT: {}", ascii_safe(tax_id)));
        }
    }
    if data.show_address {
        if let Some(address) = &data.address {
            for line in wrap(&ascii_safe(address), CHARS_PER_LINE) {
                push_line(&mut b, &line);
            }
        }
    }
    if data.show_phone {
        if let Some(phone) = &data.phone {
            push_line(&mut b, &format!("Tel: {}", ascii_safe(phone)));
        }
    }
    push_line(&mut b, "Servicio a domicilio");

    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));
    bold_on(&mut b);
    push_line(&mut b, "COMPROBANTE DE VENTA");
    bold_off(&mut b);
    if copy {
        bold_on(&mut b);
        push_line(&mut b, "*** COPIA ***");
        bold_off(&mut b);
    }
    let status_label = match data.status {
        SaleStatus::Completed => None,
        SaleStatus::Cancelled => Some("*** ANULADA ***"),
        SaleStatus::Refunded => Some("*** DEVUELTA ***"),
    };
    if let Some(label) = status_label {
        bold_on(&mut b);
        push_line(&mut b, label);
        bold_off(&mut b);
    }
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
    if data.show_cashier {
        push_line(
            &mut b,
            &format!("Vendedor: {}", ascii_safe(&data.created_by)),
        );
    }
    if let Some(reason) = &data.courtesy_reason {
        align_center(&mut b);
        bold_on(&mut b);
        push_line(&mut b, "VENTA DE CORTESIA");
        bold_off(&mut b);
        align_left(&mut b);
        for line in wrap(&format!("Motivo: {}", ascii_safe(reason)), CHARS_PER_LINE) {
            push_line(&mut b, &line);
        }
    }

    // Cliente y Documento: SIN conectar a ningún dato real, se llenan a mano.
    push_line(&mut b, "Cliente: _______________________");
    push_line(&mut b, "Documento: _____________________");

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Productos ----
    for line in &data.lines {
        push_line(
            &mut b,
            &format_item_line(line, data.currency_decimals, data.show_discounts),
        );
    }

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Totales ----
    let subtotal = if data.show_discounts {
        data.subtotal
    } else {
        data.subtotal - data.discount
    };
    push_line(&mut b, &two_col("Subtotal:", &money(subtotal)));
    if data.show_discounts && data.discount > 0 {
        push_line(
            &mut b,
            &two_col("Descuento:", &format!("-{}", money(data.discount))),
        );
    }
    if data.show_tax_breakdown {
        let label = format!("{} incl.:", ascii_safe(&data.tax_name));
        push_line(&mut b, &two_col(&label, &money(data.tax)));
    }

    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));
    bold_on(&mut b);
    push_line(&mut b, &two_col("TOTAL:", &money(data.total)));
    bold_off(&mut b);
    push_line(&mut b, &"-".repeat(CHARS_PER_LINE));

    // ---- Forma de pago ----
    if data.show_payment_method && !data.payments.is_empty() {
        align_center(&mut b);
        push_line(&mut b, "FORMA DE PAGO");
        align_left(&mut b);

        let cash: i64 = sum_by_method(data, crate::modules::sales::models::PaymentMethod::Cash);
        let transfer: i64 =
            sum_by_method(data, crate::modules::sales::models::PaymentMethod::Transfer);
        let card: i64 = sum_by_method(data, crate::modules::sales::models::PaymentMethod::Card);
        let total_received = cash + transfer + card;
        let change = (total_received - data.total).max(0);

        push_line(&mut b, &two_col("Efectivo:", &money(cash)));
        push_line(&mut b, &two_col("Transferencia:", &money(transfer)));
        push_line(&mut b, &two_col("Tarjeta:", &money(card)));
        push_line(&mut b, &"-".repeat(CHARS_PER_LINE));
        if let Some(cash_received) = data.cash_received {
            push_line(
                &mut b,
                &two_col("Efectivo recibido:", &money(cash_received)),
            );
            push_line(
                &mut b,
                &two_col(
                    "Cambio:",
                    &money(data.change_given.unwrap_or((cash_received - cash).max(0))),
                ),
            );
        } else {
            push_line(&mut b, &two_col("Total recibido:", &money(total_received)));
            push_line(&mut b, &two_col("Cambio:", &money(change)));
        }

        push_line(&mut b, &"-".repeat(CHARS_PER_LINE));
    }

    // ---- Pie ----
    align_center(&mut b);
    bold_on(&mut b);
    push_line(&mut b, "GRACIAS POR TU COMPRA");
    bold_off(&mut b);
    push_line(&mut b, &ascii_safe(&data.business_name));
    if let Some(slogan) = &data.ticket_header {
        push_line(&mut b, &ascii_safe(slogan));
    }
    if data.show_phone {
        if let Some(phone) = &data.phone {
            push_line(&mut b, &format!("Tel: {}", ascii_safe(phone)));
        }
    }
    push_line(&mut b, "Servicio a domicilio");
    if let Some(footer) = &data.ticket_footer {
        for line in wrap(&ascii_safe(footer), CHARS_PER_LINE) {
            push_line(&mut b, &line);
        }
    }

    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));

    b.extend_from_slice(b"\n\n\n");
    b.extend_from_slice(&[GS, b'V', 0x00]); // corte total
    b
}

pub fn report_lines(data: &CashReportTicketData) -> Vec<String> {
    let report = &data.report;
    let mut lines = Vec::new();

    if report.kind == "X" {
        lines.push("INFORME X - PARCIAL".to_string());
        lines.push("NO CIERRA EL TURNO".to_string());
    } else {
        lines.push("INFORME Z - CIERRE DE TURNO".to_string());
    }
    if data.is_copy {
        lines.push("*** COPIA ***".to_string());
    }
    report_pair(&mut lines, "Turno:", &format!("#{}", report.session_id));
    report_pair(&mut lines, "Abierto por:", &report.opened_by);
    report_pair(&mut lines, "Hora apertura:", &report.opened_at_local);
    if report.kind == "Z" {
        if let Some(closed_by) = &report.closed_by {
            report_pair(&mut lines, "Cerrado por:", closed_by);
        }
        if let Some(closed_at) = &report.closed_at_local {
            report_pair(&mut lines, "Hora cierre:", closed_at);
        }
    }
    report_pair(
        &mut lines,
        "Generado:",
        &report.generated_at_local,
    );

    lines.push("EFECTIVO".to_string());
    report_money_pair(
        &mut lines,
        "Apertura:",
        report.cash.opening_amount,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Ventas en efectivo:",
        report.cash.cash_sales,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Devoluciones:",
        report.cash.cash_refunds,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Ingresos manuales:",
        report.cash.manual_in,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Egresos manuales:",
        report.cash.manual_out,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Esperado:",
        report.cash.expected_amount,
        data.currency_decimals,
    );
    if report.kind == "Z" {
        if let Some(counted_amount) = report.cash.counted_amount {
            report_money_pair(
                &mut lines,
                "Contado:",
                counted_amount,
                data.currency_decimals,
            );
        }
        if let Some(difference) = report.cash.difference {
            report_money_pair(
                &mut lines,
                "Diferencia:",
                difference,
                data.currency_decimals,
            );
        }
    }

    lines.push("VENTAS".to_string());
    report_pair(&mut lines, "Cantidad:", &report.sales.count.to_string());
    report_money_pair(
        &mut lines,
        "Bruto:",
        report.sales.gross_total,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Descuentos:",
        report.sales.discount_total,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Neto:",
        report.sales.net_total,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Efectivo:",
        report.sales.by_method.cash,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Tarjeta:",
        report.sales.by_method.card,
        data.currency_decimals,
    );
    report_money_pair(
        &mut lines,
        "Transferencia:",
        report.sales.by_method.transfer,
        data.currency_decimals,
    );
    let courtesy = format!(
        "{} / {}",
        report.sales.courtesy_count,
        format_money(report.sales.courtesy_gross, data.currency_decimals)
    );
    report_pair(&mut lines, "Cortesías:", &courtesy);

    lines.push("ANULACIONES".to_string());
    report_pair(
        &mut lines,
        "Cantidad:",
        &report.voids.cancelled_count.to_string(),
    );
    report_money_pair(
        &mut lines,
        "Total anuladas:",
        report.voids.cancelled_total,
        data.currency_decimals,
    );

    lines.push("DEVOLUCIONES".to_string());
    report_pair(
        &mut lines,
        "Cantidad:",
        &report.voids.refunded_count.to_string(),
    );
    report_money_pair(
        &mut lines,
        "Total devueltas:",
        report.voids.refunded_total,
        data.currency_decimals,
    );
    lines.extend(report_wrap(
        "Tarjeta y transf. no entran al efectivo",
    ));

    lines
}

pub fn build_cash_report_ticket(data: &CashReportTicketData) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&[ESC, b'@']);
    align_center(&mut b);
    bold_on(&mut b);
    for line in report_wrap(&ascii_safe(&data.business_name)) {
        push_line(&mut b, &line);
    }
    bold_off(&mut b);
    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));
    align_left(&mut b);
    for line in report_lines(data) {
        push_line(&mut b, &line);
    }
    push_line(&mut b, &"=".repeat(CHARS_PER_LINE));
    b.extend_from_slice(b"\n\n\n");
    b.extend_from_slice(&[GS, b'V', 0x00]);
    b
}

fn report_money_pair(lines: &mut Vec<String>, label: &str, value: i64, decimals: u8) {
    report_pair(lines, label, &format_money(value, decimals));
}

fn report_pair(lines: &mut Vec<String>, label: &str, value: &str) {
    let label = report_ascii(label);
    let value = report_ascii(value);
    if label.len() + value.len() + 1 <= CHARS_PER_LINE {
        lines.push(two_col(&label, &value));
    } else {
        lines.extend(report_wrap(&label));
        lines.extend(report_wrap(&value));
    }
}

fn report_wrap(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in wrap(&report_ascii(text), CHARS_PER_LINE) {
        if line.len() <= CHARS_PER_LINE {
            lines.push(line);
        } else {
            let mut start = 0;
            while start < line.len() {
                let end = (start + CHARS_PER_LINE).min(line.len());
                lines.push(line[start..end].to_string());
                start = end;
            }
        }
    }
    lines
}

fn report_ascii(text: &str) -> String {
    ascii_safe(text).chars().filter(char::is_ascii).collect()
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

fn format_item_line(item: &TicketLine, decimals: u8, show_discounts: bool) -> String {
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
    let detail = format!(
        "{} x {}",
        item.quantity,
        format_money(item.unit_price, decimals)
    );
    let line_total = if show_discounts {
        item.unit_price * i64::from(item.quantity)
    } else {
        item.subtotal
    };
    out.push_str(&two_col(&detail, &format_money(line_total, decimals)));
    if show_discounts && item.discount > 0 {
        out.push('\n');
        out.push_str(&two_col(
            "  Desc. linea",
            &format_money(-item.discount, decimals),
        ));
    }

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

/// Amounts are stored in hundredths. Zero-decimal currencies round to the nearest unit.
fn format_money(cents: i64, decimals: u8) -> String {
    let negative = cents < 0;
    let absolute = cents.unsigned_abs();
    let (units, fraction) = if decimals == 0 {
        ((absolute + 50) / 100, None)
    } else {
        (absolute / 100, Some(absolute % 100))
    };
    let digits = units.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let grouped: String = grouped.chars().rev().collect();

    let body = match fraction {
        Some(value) => format!("{grouped},{value:02}"),
        None => grouped,
    };

    if negative && absolute != 0 {
        format!("-$ {body}")
    } else {
        format!("$ {body}")
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

#[cfg(test)]
mod tests {
    use super::{
        build_cash_report_ticket, build_sale_ticket, build_sale_ticket_with_copy,
        format_item_line, format_money, report_lines,
    };
    use crate::modules::billing::models::{CashReportTicketData, TicketData, TicketLine};
    use crate::modules::cash::models::{
        CashReportPaymentMethods, CashReportSales, CashReportTotals, CashReportVoids,
        CashSessionReport,
    };
    use crate::modules::sales::models::SaleStatus;
    use chrono::{TimeZone, Utc};

    #[test]
    fn formats_money_using_currency_precision() {
        assert_eq!(format_money(2_800_050, 0), "$ 28.001");
        assert_eq!(format_money(2_800_050, 2), "$ 28.000,50");
        assert_eq!(format_money(-50, 0), "-$ 1");
        assert_eq!(format_money(0, 0), "$ 0");
    }

    #[test]
    fn ticket_respects_hidden_output_settings() {
        let data = TicketData {
            business_name: "Tienda".into(),
            tax_id: Some("900123".into()),
            address: Some("Calle 1".into()),
            phone: Some("3010000000".into()),
            ticket_header: None,
            ticket_footer: None,
            tax_name: "IVA".into(),
            currency_decimals: 0,
            show_logo: false,
            show_tax_id: false,
            show_address: false,
            show_phone: false,
            show_cashier: false,
            show_tax_breakdown: false,
            show_discounts: false,
            show_payment_method: false,
            sale_id: 1,
            ticket_number: "V-1".into(),
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
            created_by: "Operador".into(),
            status: SaleStatus::Completed,
            courtesy_reason: Some("Apoyo a cliente".into()),
            customer_name: None,
            customer_document: None,
            lines: Vec::new(),
            payments: Vec::new(),
            subtotal: 1000,
            discount: 100,
            tax: 190,
            total: 1090,
            cash_received: None,
            change_given: None,
        };

        let ticket = String::from_utf8(build_sale_ticket(&data)).unwrap();
        for hidden in [
            "NIT:",
            "Calle 1",
            "Tel:",
            "Vendedor:",
            "Descuento:",
            "IVA incl.:",
            "FORMA DE PAGO",
        ] {
            assert!(!ticket.contains(hidden), "unexpected ticket text: {hidden}");
        }
        assert!(ticket.contains("VENTA DE CORTESIA"));
        assert!(ticket.contains("Motivo: Apoyo a cliente"));
    }

    #[test]
    fn courtesy_ticket_omits_payment_block_when_payment_method_is_enabled() {
        let data = TicketData {
            business_name: "Tienda".into(),
            tax_id: None,
            address: None,
            phone: None,
            ticket_header: None,
            ticket_footer: None,
            tax_name: "IVA".into(),
            currency_decimals: 0,
            show_logo: false,
            show_tax_id: false,
            show_address: false,
            show_phone: false,
            show_cashier: false,
            show_tax_breakdown: false,
            show_discounts: false,
            show_payment_method: true,
            sale_id: 2,
            ticket_number: "V-2".into(),
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
            created_by: "Operador".into(),
            status: SaleStatus::Completed,
            courtesy_reason: Some("Apoyo a cliente".into()),
            customer_name: None,
            customer_document: None,
            lines: Vec::new(),
            payments: Vec::new(),
            subtotal: 1000,
            discount: 1000,
            tax: 0,
            total: 0,
            cash_received: None,
            change_given: None,
        };

        let ticket = String::from_utf8(build_sale_ticket(&data)).unwrap();
        assert!(ticket.contains("VENTA DE CORTESIA"));
        assert!(ticket.contains("Motivo:"));
        assert!(!ticket.contains("Efectivo:"));
        assert!(!ticket.contains("Total recibido"));
    }

    fn cash_payment_ticket(cash_received: Option<i64>, change_given: Option<i64>) -> TicketData {
        use crate::modules::billing::models::TicketPayment;
        use crate::modules::sales::models::PaymentMethod;

        TicketData {
            business_name: "Tienda".into(),
            tax_id: None,
            address: None,
            phone: None,
            ticket_header: None,
            ticket_footer: None,
            tax_name: "IVA".into(),
            currency_decimals: 0,
            show_logo: false,
            show_tax_id: false,
            show_address: false,
            show_phone: false,
            show_cashier: false,
            show_tax_breakdown: false,
            show_discounts: false,
            show_payment_method: true,
            sale_id: 3,
            ticket_number: "V-3".into(),
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
            created_by: "Operador".into(),
            status: SaleStatus::Completed,
            courtesy_reason: None,
            customer_name: None,
            customer_document: None,
            lines: Vec::new(),
            payments: vec![TicketPayment {
                method: PaymentMethod::Cash,
                amount: 10_000,
            }],
            subtotal: 10_000,
            discount: 0,
            tax: 0,
            total: 10_000,
            cash_received,
            change_given,
        }
    }

    #[test]
    fn ticket_prints_recorded_cash_received_and_change() {
        let data = cash_payment_ticket(Some(15_000), Some(5_000));
        let ticket = String::from_utf8(build_sale_ticket(&data)).unwrap();

        assert!(ticket.contains("Efectivo recibido:"));
        assert!(ticket.contains(&format_money(15_000, 0)));
        assert!(ticket.contains("Cambio:"));
        assert!(ticket.contains(&format_money(5_000, 0)));
        assert!(!ticket.contains("Total recibido:"));
    }

    #[test]
    fn ticket_keeps_legacy_total_received_when_cash_received_is_missing() {
        let data = cash_payment_ticket(None, None);
        let ticket = String::from_utf8(build_sale_ticket(&data)).unwrap();

        assert!(ticket.contains("Total recibido:"));
        assert!(ticket.contains(&format_money(10_000, 0)));
    }

    #[test]
    fn ticket_item_prints_gross_and_line_discount_when_enabled() {
        let line = TicketLine {
            product_name: "Producto".into(),
            attributes: String::new(),
            sku: None,
            quantity: 2,
            unit_price: 10_000,
            discount: 2_000,
            subtotal: 18_000,
        };

        let rendered = format_item_line(&line, 0, true);

        assert!(rendered.contains("2 x $ 100"));
        assert!(rendered.contains("$ 200"));
        assert!(rendered.contains("Desc. linea"));
        assert!(rendered.contains("-$ 20"));
    }

    #[test]
    fn ticket_item_prints_net_without_discount_line_when_discounts_are_hidden() {
        let line = TicketLine {
            product_name: "Producto".into(),
            attributes: String::new(),
            sku: None,
            quantity: 2,
            unit_price: 10_000,
            discount: 2_000,
            subtotal: 18_000,
        };

        let rendered = format_item_line(&line, 0, false);

        assert!(rendered.contains("2 x $ 100"));
        assert!(rendered.contains("$ 180"));
        assert!(!rendered.contains("Desc. linea"));
        assert!(!rendered.contains("-$ 20"));
    }

    fn totals_ticket(show_discounts: bool) -> TicketData {
        TicketData {
            business_name: "Tienda".into(),
            tax_id: None,
            address: None,
            phone: None,
            ticket_header: None,
            ticket_footer: None,
            tax_name: "IVA".into(),
            currency_decimals: 0,
            show_logo: false,
            show_tax_id: false,
            show_address: false,
            show_phone: false,
            show_cashier: false,
            show_tax_breakdown: false,
            show_discounts,
            show_payment_method: false,
            sale_id: 4,
            ticket_number: "V-4".into(),
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
            created_by: "Operador".into(),
            status: SaleStatus::Completed,
            courtesy_reason: None,
            customer_name: None,
            customer_document: None,
            lines: Vec::new(),
            payments: Vec::new(),
            subtotal: 10_000,
            discount: 2_000,
            tax: 0,
            total: 8_000,
            cash_received: None,
            change_given: None,
        }
    }

    #[test]
    fn ticket_totals_show_gross_subtotal_and_discount_when_enabled() {
        let ticket = String::from_utf8(build_sale_ticket(&totals_ticket(true))).unwrap();

        assert!(ticket.contains("Subtotal:"));
        assert!(ticket.contains("$ 100"));
        assert!(ticket.contains("Descuento:"));
        assert!(ticket.contains("-$ 20"));
        assert!(ticket.contains("TOTAL:"));
        assert!(ticket.contains("$ 80"));
    }

    #[test]
    fn ticket_totals_show_net_subtotal_without_discount_line_when_hidden() {
        let ticket = String::from_utf8(build_sale_ticket(&totals_ticket(false))).unwrap();

        assert!(ticket.contains("Subtotal:"));
        assert!(ticket.contains("$ 80"));
        assert!(!ticket.contains("Descuento:"));
        assert!(ticket.contains("TOTAL:"));
    }

    fn cash_report_ticket(kind: &str, is_copy: bool) -> CashReportTicketData {
        CashReportTicketData {
            business_name: "Tienda de prueba".into(),
            currency_decimals: 2,
            report: CashSessionReport {
                schema_version: 1,
                kind: kind.into(),
                session_id: 12,
                opened_by: "Operador con un nombre excepcionalmente largo para probar el ancho".into(),
                closed_by: Some("Cajero".into()),
                opened_at_local: "10/10/2026 08:30".into(),
                closed_at_local: Some("10/10/2026 17:45".into()),
                generated_at_local: "10/10/2026 17:46".into(),
                cash: CashReportTotals {
                    opening_amount: i64::MAX,
                    cash_sales: 100_000,
                    cash_refunds: 10_000,
                    manual_in: 5_000,
                    manual_out: 2_000,
                    expected_amount: 93_000,
                    counted_amount: Some(93_000),
                    difference: Some(0),
                },
                sales: CashReportSales {
                    count: 8,
                    gross_total: 120_000,
                    discount_total: 20_000,
                    net_total: 100_000,
                    by_method: CashReportPaymentMethods {
                        cash: 50_000,
                        card: 30_000,
                        transfer: 20_000,
                    },
                    courtesy_count: 1,
                    courtesy_gross: 5_000,
                },
                voids: CashReportVoids {
                    cancelled_count: 1,
                    cancelled_total: 10_000,
                    refunded_count: 2,
                    refunded_total: 20_000,
                },
            },
            is_copy,
        }
    }

    #[test]
    fn sale_reprint_is_marked_as_copy_and_original_is_not() {
        let mut data = totals_ticket(false);
        let original = String::from_utf8(build_sale_ticket(&data)).unwrap();
        let copy = String::from_utf8(build_sale_ticket_with_copy(&data, true)).unwrap();
        assert!(!original.contains("*** COPIA ***"));
        assert!(copy.contains("*** COPIA ***"));

        data.status = SaleStatus::Refunded;
        let refunded = String::from_utf8(build_sale_ticket(&data)).unwrap();
        assert!(refunded.contains("*** DEVUELTA ***"));
    }

    #[test]
    fn report_lines_fit_32_columns_for_x_and_z_with_long_values() {
        let x = report_lines(&cash_report_ticket("X", false));
        assert!(x.iter().all(|line| line.len() <= 32));
        assert!(x.iter().all(|line| line.is_ascii()));
        let x_text = x.join("\n");
        assert!(x_text.contains("INFORME X - PARCIAL"));
        assert!(!x_text.to_lowercase().contains("contado"));
        assert!(!x_text.to_lowercase().contains("diferencia"));
        assert!(x_text.contains("Operador con un nombre"));

        let z = report_lines(&cash_report_ticket("Z", true));
        assert!(z.iter().all(|line| line.len() <= 32));
        assert!(z.iter().all(|line| line.is_ascii()));
        let z_text = z.join("\n");
        assert!(z_text.contains("INFORME Z - CIERRE DE TURNO"));
        assert!(z_text.contains("*** COPIA ***"));
        assert!(z_text.contains("Contado:"));
        assert!(z_text.contains("Diferencia:"));
    }

    #[test]
    fn cash_report_ticket_contains_report_lines_and_cut_command() {
        let data = cash_report_ticket("Z", false);
        let ticket = build_cash_report_ticket(&data);
        let printable = String::from_utf8(ticket.clone()).unwrap();
        assert!(printable.contains("INFORME Z - CIERRE DE TURNO"));
        assert!(ticket.ends_with(&[0x1D, b'V', 0x00]));
    }
}

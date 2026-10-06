//! Numeración del ticket de venta.
//!
//! IMPORTANTE: esto es un número interno de control (correlativo del ticket
//! impreso), NO es facturación electrónica ante la DIAN. Facturación
//! electrónica requiere resolución de numeración autorizada, firma digital
//! y transmisión a la DIAN — es un módulo aparte, mucho más grande, que
//! se puede construir después si el negocio lo necesita.
//!
//! Por ahora usamos el id autoincremental de `sales` como base del número
//! de ticket, formateado con ceros a la izquierda. Es simple, siempre único,
//! y no requiere una tabla ni columna nueva.

/// Ej: sale_id = 42 → "T-000042"
pub fn ticket_number(sale_id: i32) -> String {
    format!("T-{:06}", sale_id)
}

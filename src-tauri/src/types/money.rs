//! Política de dinero:
//! - Todo importe se guarda como i64 en centésimas de la unidad principal.
//! - `decimals` (0..=2) es la precisión visible de la moneda: COP = 0, USD = 2.
//! - Los importes calculados se redondean a esa precisión al calcularlos.

fn step(decimals: u8) -> i128 {
    10_i128.pow(2 - decimals.min(2) as u32)
}

fn clamp_i128(value: i128) -> i64 {
    value.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn round_ratio_to_currency(numerator: i128, denominator: i128, decimals: u8) -> i64 {
    let unit = step(decimals);
    let scaled_denominator = denominator * unit;
    let magnitude = numerator.abs();
    let rounded = (magnitude + scaled_denominator / 2) / scaled_denominator;
    let result = rounded * unit;
    clamp_i128(if numerator < 0 { -result } else { result })
}

/// Redondea un importe en centésimas a la precisión de la moneda
/// (mitad hacia afuera de cero).
pub fn round_to_currency(amount: i64, decimals: u8) -> i64 {
    round_ratio_to_currency(amount as i128, 1, decimals)
}

/// IVA sobre una base neta, redondeado una sola vez a la precisión de la moneda.
/// `bps`: basis points (1900 = 19 %), entre 0 y 10000.
pub fn tax_on_net(net: i64, bps: i32, decimals: u8) -> i64 {
    round_ratio_to_currency(net as i128 * bps as i128, 10_000, decimals)
}

/// IVA contenido en un importe que ya lo incluye (precios con IVA incluido).
/// `bps` debe estar entre 0 y 10000.
pub fn tax_included_in(gross: i64, bps: i32, decimals: u8) -> i64 {
    round_ratio_to_currency(gross as i128 * bps as i128, 10_000 + bps as i128, decimals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cop_rounds_to_whole_pesos() {
        assert_eq!(round_to_currency(19_019, 0), 19_000);
        assert_eq!(round_to_currency(19_050, 0), 19_100);
        assert_eq!(round_to_currency(19_019, 2), 19_019);
        assert_eq!(round_to_currency(-19_050, 0), -19_100);
    }

    #[test]
    fn tax_is_rounded_once_to_currency_precision() {
        assert_eq!(tax_on_net(100_100, 1900, 0), 19_000);
        assert_eq!(tax_on_net(100_100, 1900, 2), 19_019);
        assert_eq!(tax_on_net(260, 1900, 0), 0);
    }

    #[test]
    fn tax_included_in_gross_is_rounded_once() {
        assert_eq!(tax_included_in(11_900, 1900, 2), 1_900);
        assert_eq!(tax_included_in(11_900, 1900, 0), 1_900);
    }
}

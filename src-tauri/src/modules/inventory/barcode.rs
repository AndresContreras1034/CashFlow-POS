/// Calcula el dígito verificador EAN-13 para los primeros 12 dígitos.
pub fn ean13_check_digit(first12: &str) -> Option<u32> {
    if first12.len() != 12 || !first12.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    let sum: u32 = first12
        .chars()
        .enumerate()
        .map(|(index, character)| {
            let digit = character.to_digit(10).unwrap();
            if index % 2 == 0 {
                digit
            } else {
                digit * 3
            }
        })
        .sum();
    Some((10 - sum % 10) % 10)
}

/// Genera un EAN-13 interno: prefijo 20, secuencia de 10 dígitos y verificador.
pub fn internal_ean13(sequence: i64) -> Option<String> {
    if !(0..10_000_000_000).contains(&sequence) {
        return None;
    }

    let base = format!("20{:010}", sequence);
    let check_digit = ean13_check_digit(&base)?;
    Some(format!("{}{}", base, check_digit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_digit_known_value() {
        assert_eq!(ean13_check_digit("400638133393"), Some(1));
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(ean13_check_digit("12345"), None);
        assert_eq!(ean13_check_digit("40063813339a"), None);
    }

    #[test]
    fn internal_code_shape() {
        let code = internal_ean13(1).unwrap();
        assert_eq!(code.len(), 13);
        assert!(code.starts_with("20"));
        assert_eq!(
            ean13_check_digit(&code[..12]).unwrap().to_string(),
            code[12..]
        );
    }

    #[test]
    fn rejects_out_of_range_sequence() {
        assert_eq!(internal_ean13(-1), None);
        assert_eq!(internal_ean13(10_000_000_000), None);
    }
}

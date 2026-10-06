use super::barcode::ean13_check_digit;
use super::dto::LabelData;
use super::export::format_attributes;

const COLS: usize = 32;
pub const MAX_COPIES: u32 = 100;

fn fold_char(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        'Á' | 'À' | 'Ä' | 'Â' => 'A',
        'É' | 'È' | 'Ë' | 'Ê' => 'E',
        'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
        'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
        'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
        'Ñ' => 'N',
        'Ç' => 'C',
        c if c.is_ascii() && !c.is_ascii_control() => c,
        c if c.is_whitespace() => ' ',
        _ => '?',
    }
}

fn fold(s: &str) -> String {
    s.chars().map(fold_char).collect()
}

fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.len() > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let rest = word.split_off(width);
            lines.push(word);
            word = rest;
        }
        if current.is_empty() {
            current = word;
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(&word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            if last.len() > width - 2 {
                last.truncate(width - 2);
            }
            last.push_str("..");
        }
    }
    lines
}

fn format_price(cents: i64) -> String {
    let pesos = (cents / 100).to_string();
    let mut out = String::new();
    for (i, ch) in pesos.chars().enumerate() {
        if i > 0 && (pesos.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    format!("${}", out)
}

fn push_barcode(buf: &mut Vec<u8>, code: &str) -> Result<(), String> {
    let code = code.trim();

    let is_ean13 = code.len() == 13
        && code.bytes().all(|b| b.is_ascii_digit())
        && ean13_check_digit(&code[..12]) == Some((code.as_bytes()[12] - b'0') as u32);

    if is_ean13 {
        buf.extend_from_slice(b"\x1D\x77\x02");
        buf.extend_from_slice(b"\x1D\x6B\x43\x0D");
        buf.extend_from_slice(code.as_bytes());
        return Ok(());
    }

    if code.is_empty() || code.len() > 30 {
        return Err("El código de barras debe tener entre 1 y 30 caracteres".into());
    }
    if !code.bytes().all(|b| (0x20..=0x7E).contains(&b)) || code.contains('{') {
        return Err("El código de barras tiene caracteres que no se pueden imprimir".into());
    }

    let module = if code.len() <= 12 { 2u8 } else { 1u8 };
    buf.extend_from_slice(&[0x1D, 0x77, module]);
    buf.extend_from_slice(&[0x1D, 0x6B, 0x49, (code.len() + 2) as u8]);
    buf.extend_from_slice(b"{B");
    buf.extend_from_slice(code.as_bytes());
    Ok(())
}

pub fn build_labels(data: &LabelData, copies: u32) -> Result<Vec<u8>, String> {
    if copies == 0 || copies > MAX_COPIES {
        return Err(format!("Las copias deben estar entre 1 y {}", MAX_COPIES));
    }
    let barcode = data
        .barcode
        .as_deref()
        .map(str::trim)
        .filter(|barcode| !barcode.is_empty())
        .ok_or_else(|| {
            "La variante no tiene código de barras. Genera uno en el formulario de la variante"
                .to_string()
        })?;

    let mut one: Vec<u8> = Vec::new();
    one.extend_from_slice(b"\x1B\x61\x01");

    one.extend_from_slice(b"\x1B\x45\x01");
    for line in wrap(&fold(&data.product_name), COLS, 2) {
        one.extend_from_slice(line.as_bytes());
        one.push(b'\n');
    }
    one.extend_from_slice(b"\x1B\x45\x00");

    let attrs = fold(&format_attributes(&data.attributes));
    if !attrs.trim().is_empty() {
        for line in wrap(&attrs, COLS, 2) {
            one.extend_from_slice(line.as_bytes());
            one.push(b'\n');
        }
    }

    one.extend_from_slice(b"\x1D\x21\x11");
    one.extend_from_slice(format_price(data.price).as_bytes());
    one.push(b'\n');
    one.extend_from_slice(b"\x1D\x21\x00");

    one.extend_from_slice(b"\x1D\x68\x3C\x1D\x48\x02");
    push_barcode(&mut one, barcode)?;
    one.extend_from_slice(b"\n\n\n");
    one.extend_from_slice(b"\x1D\x56\x00");

    let mut out = Vec::with_capacity(2 + one.len() * copies as usize);
    out.extend_from_slice(b"\x1B\x40");
    for _ in 0..copies {
        out.extend_from_slice(&one);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data(barcode: Option<&str>) -> LabelData {
        LabelData {
            product_name: "Perfume Rosa".into(),
            attributes: json!({ "Volumen": "100ml" }),
            barcode: barcode.map(String::from),
            price: 2_800_000,
        }
    }

    fn count(buf: &[u8], pat: &[u8]) -> usize {
        buf.windows(pat.len())
            .filter(|window| *window == pat)
            .count()
    }

    #[test]
    fn folds_accents_and_controls() {
        assert_eq!(fold("Ñandú Café"), "Nandu Cafe");
        assert_eq!(fold("A\u{1b}B"), "A?B");
    }

    #[test]
    fn wraps_and_truncates() {
        assert_eq!(wrap("uno dos tres", 7, 2), vec!["uno dos", "tres"]);
        assert_eq!(wrap("aaaa bbbb cccc dddd", 4, 2), vec!["aaaa", "bb.."]);
    }

    #[test]
    fn price_format() {
        assert_eq!(format_price(2_800_000), "$28.000");
        assert_eq!(format_price(123_456_789), "$1.234.567");
        assert_eq!(format_price(0), "$0");
    }

    #[test]
    fn valid_ean13_uses_native_command() {
        let out = build_labels(&data(Some("2000000000015")), 1).unwrap();
        assert_eq!(count(&out, b"\x1D\x6B\x43\x0D2000000000015"), 1);
    }

    #[test]
    fn invalid_check_digit_falls_back_to_code128() {
        let out = build_labels(&data(Some("2000000000016")), 1).unwrap();
        assert_eq!(count(&out, b"\x1D\x6B\x49\x0F{B2000000000016"), 1);
    }

    #[test]
    fn missing_barcode_is_an_error() {
        assert!(build_labels(&data(None), 1).is_err());
    }

    #[test]
    fn copies_are_bounded_and_repeated() {
        assert!(build_labels(&data(Some("2000000000015")), 0).is_err());
        assert!(build_labels(&data(Some("2000000000015")), 101).is_err());
        let three = build_labels(&data(Some("2000000000015")), 3).unwrap();
        assert_eq!(count(&three, b"\x1D\x56\x00"), 3);
    }
}

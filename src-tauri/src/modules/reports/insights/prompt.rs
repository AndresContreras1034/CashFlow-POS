use super::advice::AdviceResponse;
use serde_json::Value;

pub const SYSTEM_PROMPT: &str = r#"Eres un asesor comercial de una tienda pequeña. Lee el contexto JSON ya calculado.
Responde en español y solo con JSON válido. No inventes, recalcules ni cites datos que no estén en el contexto. Máximo 2 consejos, breves y accionables; incluye una cita exacta por consejo. No marques datos insuficientes solo porque falte costo en algunos productos: utiliza las cifras disponibles y menciona esa limitación. Si propones promoción, usa solo tipos y descuentos disponibles y menciona el punto de equilibrio. Marca insufficient_data=true solo cuando no exista ninguna cifra útil que puedas citar.
Esquema exacto: {"insufficient_data":bool,"advice":[{"observation":str,"context":str,"proposal":str,"inventory_action":str|null,"promo":{"alias":str,"kind":"two_for_one|second_unit_off|direct_discount","second_off_pct":int|null,"direct_off_pct":int|null}|null,"cited":[{"path":"JSON Pointer del contexto","value":valor exacto}]}]}"#;

pub fn user_message(input: &Value) -> String {
    format!(
        "Contexto (JSON):\n{}\n\nDevuelve solo el objeto JSON solicitado. Mantén cada texto breve.",
        serde_json::to_string(input).unwrap_or_default()
    )
}

pub fn parse_response(raw: &str) -> Result<AdviceResponse, serde_json::Error> {
    let t = raw.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t).trim();
    serde_json::from_str(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{"insufficient_data":false,"advice":[{"observation":"o","context":"c","proposal":"p","cited":[{"path":"/pace/days","value":5}]}]}"#;

    #[test]
    fn parses_plain_json() {
        let r = parse_response(OK).unwrap();
        assert_eq!(r.advice.len(), 1);
        assert!(r.advice[0].promo.is_none());
    }

    #[test]
    fn parses_fenced_json_and_rejects_garbage() {
        let fenced = format!("```json\n{OK}\n```");
        assert!(parse_response(&fenced).is_ok());
        assert!(parse_response("no es json").is_err());
    }
}

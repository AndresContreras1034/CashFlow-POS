use super::advice::AdviceResponse;
use serde_json::Value;

pub const SYSTEM_PROMPT: &str = "Eres un asesor comercial para un comercio pequeño. Recibes un JSON con cifras YA CALCULADAS. Reglas: 1) NUNCA calcules ni inventes cifras; solo cita valores del JSON y en 'cited' indica ruta (JSON Pointer) y valor exacto. 2) Máximo 3 consejos por impacto, español claro. 3) Cada consejo: observación, contexto, propuesta, acción de inventario si aplica. 4) Para promociones indica solo tipo y porcentajes; nunca propongas vender bajo costo ni presentes algo como conveniente solo por estar permitido: menciona el punto de equilibrio. 5) Si faltan datos marca insufficient_data=true. 6) Solo sugieres. Responde ÚNICAMENTE un objeto JSON con este esquema: {\"insufficient_data\": bool, \"advice\": [{\"observation\": str, \"context\": str, \"proposal\": str, \"inventory_action\": str|null, \"promo\": {\"alias\": str, \"kind\": \"two_for_one|second_unit_off|direct_discount\", \"second_off_pct\": int|null, \"direct_off_pct\": int|null}|null, \"cited\": [{\"path\": \"/ruta/json/pointer\", \"value\": <valor exacto del JSON>}]}]}";

pub fn user_message(input: &Value) -> String {
    format!(
        "Contexto (JSON):\n{}\n\nResponde solo con el JSON del esquema.",
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

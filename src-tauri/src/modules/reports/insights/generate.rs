use super::policy::{self, BlockReason, Decision};
use super::provider::{AiError, AiProvider};
use super::store::{self, StoredAdvice};
use super::{prompt, validate::validate};
use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;

#[derive(Debug)]
pub enum GenError {
    Blocked(BlockReason),
    Ai(AiError),
    Db(sqlx::Error),
    InvalidReply,
    BadInput(&'static str),
}

impl From<sqlx::Error> for GenError {
    fn from(e: sqlx::Error) -> Self {
        GenError::Db(e)
    }
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenError::Blocked(BlockReason::Disabled) => {
                write!(f, "La IA está desactivada en Ajustes")
            }
            GenError::Blocked(BlockReason::TokenCap) => {
                write!(f, "Se alcanzó el tope diario de consumo de IA")
            }
            GenError::Blocked(BlockReason::RegenLimit) => write!(f, "Ya regeneraste 3 veces hoy"),
            GenError::Blocked(BlockReason::SameInput) => {
                write!(f, "Los datos no han cambiado desde el último consejo")
            }
            GenError::Ai(e) => write!(f, "{e}"),
            GenError::Db(e) => write!(f, "Error de base de datos: {e}"),
            GenError::InvalidReply => write!(f, "La IA devolvió una respuesta que no se pudo leer"),
            GenError::BadInput(w) => write!(f, "Contexto inválido: falta {w}"),
        }
    }
}

/// `ctx` es el JSON de get_commercial_context (incluye "as_of" y "fingerprint").
pub async fn generate<P: AiProvider>(
    pool: &PgPool,
    provider: &P,
    scope: &str,
    ctx: &Value,
) -> Result<StoredAdvice, GenError> {
    if !policy::valid_scope(scope) {
        return Err(GenError::BadInput("scope"));
    }
    let date = ctx
        .get("as_of")
        .and_then(Value::as_str)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .ok_or(GenError::BadInput("as_of"))?;
    let hash = ctx
        .get("fingerprint")
        .and_then(Value::as_str)
        .ok_or(GenError::BadInput("fingerprint"))?
        .to_string();
    let mut input = ctx.clone();
    if let Some(o) = input.as_object_mut() {
        o.remove("fingerprint");
    }
    let min_bps = input
        .pointer("/settings/min_margin_bps")
        .and_then(Value::as_i64)
        .unwrap_or(1_500);

    let (enabled, _model, cap) = store::load_settings(pool).await?;
    let state = store::day_state(pool, date, scope).await?;
    let regen_index = match policy::decide(enabled, i64::from(cap), &state, &hash) {
        Decision::Generate { regen_index } => regen_index,
        Decision::Blocked(r) => return Err(GenError::Blocked(r)),
    };

    let reply = provider
        .complete(prompt::SYSTEM_PROMPT, &prompt::user_message(&input))
        .await
        .map_err(GenError::Ai)?;
    let model = provider.model().to_string();

    let parsed = match prompt::parse_response(&reply.content) {
        Ok(p) => p,
        Err(_) => {
            store::insert(
                pool,
                date,
                scope,
                regen_index,
                &hash,
                &json!({"error": "invalid_json"}),
                &model,
                reply.prompt_tokens,
                reply.completion_tokens,
                "failed",
            )
            .await?;
            return Err(GenError::InvalidReply);
        }
    };

    let checked = validate(parsed, &input, min_bps);
    let content = serde_json::to_value(&checked).map_err(|_| GenError::InvalidReply)?;
    let id = store::insert(
        pool,
        date,
        scope,
        regen_index,
        &hash,
        &content,
        &model,
        reply.prompt_tokens,
        reply.completion_tokens,
        "ok",
    )
    .await?;
    Ok(StoredAdvice {
        id,
        local_date: date,
        scope: scope.to_string(),
        regen_index,
        content,
        model,
        generated_at: Utc::now(),
    })
}

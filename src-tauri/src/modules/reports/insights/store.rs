use super::policy::DayState;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone, Serialize)]
pub struct StoredAdvice {
    pub id: i64,
    pub local_date: NaiveDate,
    pub scope: String,
    pub regen_index: i32,
    pub content: Value,
    pub model: String,
    pub generated_at: DateTime<Utc>,
}

pub async fn load_settings(pool: &PgPool) -> Result<(bool, String, i32), sqlx::Error> {
    let r = sqlx::query("SELECT enabled, model, daily_token_cap FROM ai_settings WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok((r.get("enabled"), r.get("model"), r.get("daily_token_cap")))
}

pub async fn day_state(
    pool: &PgPool,
    date: NaiveDate,
    scope: &str,
) -> Result<DayState, sqlx::Error> {
    let tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0)::bigint \
         FROM ai_advice_daily WHERE local_date = $1",
    )
    .bind(date)
    .fetch_one(pool)
    .await?;
    let last_index: Option<i32> = sqlx::query_scalar(
        "SELECT regen_index FROM ai_advice_daily \
         WHERE local_date = $1 AND scope = $2 ORDER BY regen_index DESC LIMIT 1",
    )
    .bind(date)
    .bind(scope)
    .fetch_optional(pool)
    .await?;
    let last_ok_hash: Option<String> = sqlx::query_scalar(
        "SELECT input_hash FROM ai_advice_daily \
         WHERE local_date = $1 AND scope = $2 AND status = 'ok' \
         ORDER BY regen_index DESC LIMIT 1",
    )
    .bind(date)
    .bind(scope)
    .fetch_optional(pool)
    .await?;
    Ok(DayState {
        tokens_today: tokens,
        last_index,
        last_ok_hash,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &PgPool,
    date: NaiveDate,
    scope: &str,
    regen_index: i32,
    input_hash: &str,
    content: &Value,
    model: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    status: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO ai_advice_daily \
         (local_date, scope, regen_index, input_hash, content, model, prompt_tokens, completion_tokens, status) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(date)
    .bind(scope)
    .bind(regen_index)
    .bind(input_hash)
    .bind(content)
    .bind(model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(status)
    .fetch_one(pool)
    .await
}

/// Lectura pura (cero llamadas a la API): el último consejo ok del scope, de hoy o anterior.
pub async fn latest_ok(pool: &PgPool, scope: &str) -> Result<Option<StoredAdvice>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, local_date, scope, regen_index, content, model, generated_at \
         FROM ai_advice_daily WHERE scope = $1 AND status = 'ok' \
         ORDER BY local_date DESC, regen_index DESC LIMIT 1",
    )
    .bind(scope)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| StoredAdvice {
        id: r.get("id"),
        local_date: r.get("local_date"),
        scope: r.get("scope"),
        regen_index: r.get("regen_index"),
        content: r.get("content"),
        model: r.get("model"),
        generated_at: r.get("generated_at"),
    }))
}

pub async fn set_feedback(pool: &PgPool, id: i64, feedback: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE ai_advice_daily SET feedback = $2 WHERE id = $1")
        .bind(id)
        .bind(feedback)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn token_usage(pool: &PgPool, id: i64) -> Result<(i32, i32), sqlx::Error> {
    sqlx::query_as("SELECT prompt_tokens, completion_tokens FROM ai_advice_daily WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

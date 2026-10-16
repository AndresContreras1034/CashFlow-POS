use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct AiReply {
    pub content: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

#[derive(Debug)]
pub enum AiError {
    Network(String),
    Status(u16),
    BadBody(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::Network(e) => write!(f, "Error de red con el servicio de IA: {e}"),
            AiError::Status(s) => write!(f, "El servicio de IA respondió con estado {s}"),
            AiError::BadBody(e) => write!(f, "Respuesta inválida del servicio de IA: {e}"),
        }
    }
}

impl std::error::Error for AiError {}

#[allow(async_fn_in_trait)]
pub trait AiProvider {
    fn model(&self) -> &str;
    async fn complete(&self, system: &str, user: &str) -> Result<AiReply, AiError>;
}

pub fn parse_reply(body: &Value) -> Result<AiReply, AiError> {
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AiError::BadBody("sin contenido".into()))?
        .to_string();
    let tokens = |p: &str| {
        body.pointer(p)
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(0, i32::MAX as i64) as i32
    };
    Ok(AiReply {
        content,
        prompt_tokens: tokens("/usage/prompt_tokens"),
        completion_tokens: tokens("/usage/completion_tokens"),
    })
}

/// Cliente OpenAI-compatible. No implementa Debug a propósito: contiene la clave.
pub struct DeepSeek {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl DeepSeek {
    pub fn new(api_key: String, model: String) -> Result<Self, AiError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| AiError::Network(e.to_string()))?;
        Ok(Self {
            client,
            api_key,
            model,
            base_url: "https://api.deepseek.com".into(),
        })
    }
}

impl AiProvider for DeepSeek {
    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, system: &str, user: &str) -> Result<AiReply, AiError> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "response_format": {"type": "json_object"},
            "max_tokens": 1500,
            "temperature": 0.2,
            "stream": false
        });
        let url = format!("{}/chat/completions", self.base_url);
        let mut last = AiError::Network("sin intentos".into());
        for attempt in 0..2 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            let resp = match self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last = AiError::Network(e.without_url().to_string());
                    continue;
                }
            };
            let status = resp.status();
            if status.is_success() {
                let v: Value = resp
                    .json()
                    .await
                    .map_err(|e| AiError::BadBody(e.to_string()))?;
                return parse_reply(&v);
            }
            last = AiError::Status(status.as_u16());
            if !(status.as_u16() == 429 || status.is_server_error()) {
                break;
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_and_usage() {
        let v = json!({
            "choices": [{"message": {"content": "{}"}}],
            "usage": {"prompt_tokens": 120, "completion_tokens": 30}
        });
        let r = parse_reply(&v).unwrap();
        assert_eq!(r.content, "{}");
        assert_eq!((r.prompt_tokens, r.completion_tokens), (120, 30));
    }

    #[test]
    fn rejects_empty_content() {
        let v = json!({"choices": [{"message": {"content": "  "}}]});
        assert!(parse_reply(&v).is_err());
    }
}

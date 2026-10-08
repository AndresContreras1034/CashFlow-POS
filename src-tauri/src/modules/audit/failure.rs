use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository,
};

const MAX_MESSAGE_CHARS: usize = 500;

/// Qué operación falló. Lo arma el handler.
pub struct FailureContext<'a> {
    pub module: AuditModule,
    pub action: &'a str,
    pub entity_type: Option<&'a str>,
    pub entity_id: Option<String>,
    pub actor: Option<String>,
}

/// Solo los fallos técnicos se auditan; los rechazos de validación y los
/// "no encontrado" son respuestas esperadas del negocio.
fn technical_details(error: &AppError) -> Option<(&'static str, String)> {
    match error {
        AppError::Database(sqlx_error) => {
            let message = match sqlx_error.as_database_error() {
                Some(db) => format!(
                    "Error de base de datos (código {}, constraint {})",
                    db.code().as_deref().unwrap_or("?"),
                    db.constraint().unwrap_or("-")
                ),
                None => format!("Error de base de datos: {sqlx_error}"),
            };
            Some(("database", message))
        }
        AppError::Internal(message) => Some((
            "internal",
            message.chars().take(MAX_MESSAGE_CHARS).collect(),
        )),
        AppError::Validation(_) | AppError::NotFound(_) => None,
    }
}

/// Registra el fallo en una conexión aparte (sobrevive al rollback de la
/// operación). Nunca propaga errores: si no se puede auditar, solo se loguea.
async fn record_failure(pool: &PgPool, ctx: &FailureContext<'_>, error: &AppError) {
    let Some((kind, message)) = technical_details(error) else {
        return;
    };

    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(acquire_error) => {
            tracing::error!(
                "No se pudo auditar el fallo de {}: {}",
                ctx.action,
                acquire_error
            );
            return;
        }
    };

    let event = NewAuditEvent {
        correlation_id: Some(Uuid::new_v4()),
        category: AuditCategory::Error,
        module: ctx.module,
        action: ctx.action.to_string(),
        outcome: AuditOutcome::Failure,
        actor: ctx.actor.clone(),
        entity_type: ctx.entity_type.map(str::to_string),
        entity_id: ctx.entity_id.clone(),
        summary: format!("Falló la operación «{}»", ctx.action),
        changes: None,
        metadata: Some(json!({ "error_kind": kind })),
        error_message: Some(message),
    };

    if let Err(insert_error) = repository::insert_event(&mut *conn, event).await {
        tracing::error!(
            "No se pudo auditar el fallo de {}: {}",
            ctx.action,
            insert_error
        );
    }
}

/// Envuelve el resultado de un service en un handler: audita el fallo
/// técnico (si lo es) y devuelve el mismo `String` de error que antes.
pub async fn finish<T>(
    pool: &PgPool,
    ctx: FailureContext<'_>,
    result: Result<T, AppError>,
) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            record_failure(pool, &ctx, &error).await;
            Err(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_rejections_are_not_audited() {
        assert!(technical_details(&AppError::validation("monto inválido")).is_none());
        assert!(technical_details(&AppError::not_found("no existe")).is_none());
    }

    #[test]
    fn technical_errors_are_audited() {
        let (kind, _) = technical_details(&AppError::Database(sqlx::Error::PoolTimedOut)).unwrap();
        assert_eq!(kind, "database");

        let (kind, message) = technical_details(&AppError::internal("falló")).unwrap();
        assert_eq!(kind, "internal");
        assert_eq!(message, "falló");
    }

    #[test]
    fn internal_messages_are_truncated() {
        let long = "x".repeat(900);
        let (_, message) = technical_details(&AppError::internal(&long)).unwrap();
        assert_eq!(message.chars().count(), MAX_MESSAGE_CHARS);
    }
}

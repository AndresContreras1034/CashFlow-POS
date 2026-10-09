use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::modules::audit::{
    dto::{AuditEventFilterDto, NewAuditEvent},
    models::{AuditCategory, AuditEvent, AuditModule, AuditOutcome},
};
use crate::modules::inventory::dto::SortDir;

/// Tope de eventos que devuelve `list_by_correlation`. Una operación normal
/// (venta, cierre de caja) tiene unos pocos eventos; el tope protege ante una
/// importación con muchísimas filas rechazadas. Para recorrer más, usar
/// `list_events` con el filtro `correlation_id` y paginación.
const CORRELATION_LIMIT: i64 = 1000;

// ============================================================
// HELPERS
// ============================================================

/// Texto de un filtro: sin espacios sobrantes y `None` si queda vacío.
fn clean(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Patrón ILIKE para `search`, escapando `\`, `%` y `_` para que el texto
/// del usuario se busque literalmente.
fn search_pattern(search: &Option<String>) -> Option<String> {
    let text = clean(search)?;
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

// ============================================================
// ESCRITURA
// ============================================================

/// Inserta un evento usando la conexión/transacción del llamador.
///
/// NO abre transacción propia y NO hace COMMIT, igual que `apply_stock_out`.
/// - Evento que debe desaparecer si la operación hace rollback: pasar la
///   transacción de la operación.
/// - Evento de fallo, que debe sobrevivir al rollback: pasar una conexión
///   aparte (`pool.acquire()`).
pub async fn insert_event(
    conn: &mut PgConnection,
    event: NewAuditEvent,
) -> Result<AuditEvent, sqlx::Error> {
    let inserted = sqlx::query_as!(
        AuditEvent,
        r#"INSERT INTO audit_events
			   (correlation_id, category, module, action, outcome, actor,
				entity_type, entity_id, summary, changes, metadata, error_message)
		   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
		   RETURNING
			   id, created_at, correlation_id,
			   category AS "category: AuditCategory",
			   module   AS "module: AuditModule",
			   action,
			   outcome  AS "outcome: AuditOutcome",
			   actor, entity_type, entity_id, summary,
			   changes, metadata, error_message"#,
        event.correlation_id,
        event.category as AuditCategory,
        event.module as AuditModule,
        event.action,
        event.outcome as AuditOutcome,
        event.actor,
        event.entity_type,
        event.entity_id,
        event.summary,
        event.changes,
        event.metadata,
        event.error_message
    )
    .fetch_one(&mut *conn)
    .await?;

    log_feed(&inserted);
    Ok(inserted)
}

/// Texto de una línea del feed: `summary (actor)`, o solo `summary` sin actor.
fn feed_message(summary: &str, actor: Option<&str>) -> String {
    match actor.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("{summary} ({name})"),
        None => summary.to_string(),
    }
}

/// Línea en el CMD por cada evento de negocio guardado.
/// Los fallos técnicos no se imprimen aquí: ya los captura Developer Mode.
fn log_feed(event: &AuditEvent) {
    if matches!(event.outcome, AuditOutcome::Failure) {
        return;
    }
    let message = feed_message(&event.summary, event.actor.as_deref());
    tracing::info!(target: "pos_lib::audit", ok = true, "{}", message);
}

// ============================================================
// CONSULTA
// ============================================================

pub async fn get_event(pool: &PgPool, id: i64) -> Result<Option<AuditEvent>, sqlx::Error> {
    sqlx::query_as!(
        AuditEvent,
        r#"SELECT
			   id, created_at, correlation_id,
			   category AS "category: AuditCategory",
			   module   AS "module: AuditModule",
			   action,
			   outcome  AS "outcome: AuditOutcome",
			   actor, entity_type, entity_id, summary,
			   changes, metadata, error_message
		   FROM audit_events
		   WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Todos los eventos de una operación, del más antiguo al más reciente
/// ("Ver operación completa"). Máximo `CORRELATION_LIMIT` eventos.
pub async fn list_by_correlation(
    pool: &PgPool,
    correlation_id: Uuid,
) -> Result<Vec<AuditEvent>, sqlx::Error> {
    sqlx::query_as!(
        AuditEvent,
        r#"SELECT
			   id, created_at, correlation_id,
			   category AS "category: AuditCategory",
			   module   AS "module: AuditModule",
			   action,
			   outcome  AS "outcome: AuditOutcome",
			   actor, entity_type, entity_id, summary,
			   changes, metadata, error_message
		   FROM audit_events
		   WHERE correlation_id = $1
		   ORDER BY created_at ASC, id ASC
		   LIMIT $2"#,
        correlation_id,
        CORRELATION_LIMIT
    )
    .fetch_all(pool)
    .await
}

/// Listado con filtros, búsqueda, orden por fecha y paginación.
///
/// El orden usa dos queries (asc / desc) en lugar de un `CASE` en el
/// `ORDER BY`: con `CASE` PostgreSQL no puede recorrer el índice
/// `idx_audit_events_created` y ordenaría toda la tabla en cada página.
pub async fn list_events(
    pool: &PgPool,
    filter: &AuditEventFilterDto,
) -> Result<(Vec<AuditEvent>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1).saturating_mul(page_size);

    let action = clean(&filter.action);
    let entity_type = clean(&filter.entity_type);
    let entity_id = clean(&filter.entity_id);
    let search = search_pattern(&filter.search);

    // Zona de Ajustes: define dónde empieza y termina cada día del filtro.
    let tz: String = sqlx::query_scalar!(
        r#"SELECT COALESCE(
               (SELECT timezone FROM app_settings WHERE id = 1),
               current_setting('TimeZone')
           ) AS "tz!""#
    )
    .fetch_one(pool)
    .await?;

    let rows = match filter.sort_dir.unwrap_or(SortDir::Desc) {
        SortDir::Desc => {
            sqlx::query_as!(
                AuditEvent,
                r#"SELECT
					   id, created_at, correlation_id,
					   category AS "category: AuditCategory",
					   module   AS "module: AuditModule",
					   action,
					   outcome  AS "outcome: AuditOutcome",
					   actor, entity_type, entity_id, summary,
					   changes, metadata, error_message
				   FROM audit_events
				   WHERE ($1::audit_module   IS NULL OR module = $1)
					 AND ($2::audit_category IS NULL OR category = $2)
					 AND ($3::audit_outcome  IS NULL OR outcome = $3)
					 AND ($4::TEXT IS NULL OR action = $4)
					 AND ($5::TEXT IS NULL OR entity_type = $5)
					 AND ($6::TEXT IS NULL OR entity_id = $6)
					 AND ($7::UUID IS NULL OR correlation_id = $7)
					 AND ($8::TEXT IS NULL
						  OR summary ILIKE $8 OR action ILIKE $8
						  OR entity_id ILIKE $8 OR error_message ILIKE $8)
                     AND ($9::DATE  IS NULL OR created_at >= ($9::DATE)::timestamp AT TIME ZONE $13)
                     AND ($10::DATE IS NULL OR created_at <  ($10::DATE + 1)::timestamp AT TIME ZONE $13)
				   ORDER BY created_at DESC, id DESC
				   LIMIT $11 OFFSET $12"#,
                filter.module as Option<AuditModule>,
                filter.category as Option<AuditCategory>,
                filter.outcome as Option<AuditOutcome>,
                action,
                entity_type,
                entity_id,
                filter.correlation_id,
                search,
                filter.date_from,
                filter.date_to,
                page_size,
                offset,
                tz
            )
            .fetch_all(pool)
            .await?
        }
        SortDir::Asc => {
            sqlx::query_as!(
                AuditEvent,
                r#"SELECT
					   id, created_at, correlation_id,
					   category AS "category: AuditCategory",
					   module   AS "module: AuditModule",
					   action,
					   outcome  AS "outcome: AuditOutcome",
					   actor, entity_type, entity_id, summary,
					   changes, metadata, error_message
				   FROM audit_events
				   WHERE ($1::audit_module   IS NULL OR module = $1)
					 AND ($2::audit_category IS NULL OR category = $2)
					 AND ($3::audit_outcome  IS NULL OR outcome = $3)
					 AND ($4::TEXT IS NULL OR action = $4)
					 AND ($5::TEXT IS NULL OR entity_type = $5)
					 AND ($6::TEXT IS NULL OR entity_id = $6)
					 AND ($7::UUID IS NULL OR correlation_id = $7)
					 AND ($8::TEXT IS NULL
						  OR summary ILIKE $8 OR action ILIKE $8
						  OR entity_id ILIKE $8 OR error_message ILIKE $8)
                     AND ($9::DATE  IS NULL OR created_at >= ($9::DATE)::timestamp AT TIME ZONE $13)
                     AND ($10::DATE IS NULL OR created_at <  ($10::DATE + 1)::timestamp AT TIME ZONE $13)
				   ORDER BY created_at ASC, id ASC
				   LIMIT $11 OFFSET $12"#,
                filter.module as Option<AuditModule>,
                filter.category as Option<AuditCategory>,
                filter.outcome as Option<AuditOutcome>,
                action,
                entity_type,
                entity_id,
                filter.correlation_id,
                search,
                filter.date_from,
                filter.date_to,
                page_size,
                offset,
                tz
            )
            .fetch_all(pool)
            .await?
        }
    };

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM audit_events
		   WHERE ($1::audit_module   IS NULL OR module = $1)
			 AND ($2::audit_category IS NULL OR category = $2)
			 AND ($3::audit_outcome  IS NULL OR outcome = $3)
			 AND ($4::TEXT IS NULL OR action = $4)
			 AND ($5::TEXT IS NULL OR entity_type = $5)
			 AND ($6::TEXT IS NULL OR entity_id = $6)
			 AND ($7::UUID IS NULL OR correlation_id = $7)
			 AND ($8::TEXT IS NULL
				  OR summary ILIKE $8 OR action ILIKE $8
				  OR entity_id ILIKE $8 OR error_message ILIKE $8)
             AND ($9::DATE  IS NULL OR created_at >= ($9::DATE)::timestamp AT TIME ZONE $11)
             AND ($10::DATE IS NULL OR created_at <  ($10::DATE + 1)::timestamp AT TIME ZONE $11)"#,
        filter.module as Option<AuditModule>,
        filter.category as Option<AuditCategory>,
        filter.outcome as Option<AuditOutcome>,
        action,
        entity_type,
        entity_id,
        filter.correlation_id,
        search,
        filter.date_from,
        filter.date_to,
        tz
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_pattern_ignores_blank_text() {
        assert_eq!(search_pattern(&None), None);
        assert_eq!(search_pattern(&Some("   ".to_string())), None);
    }

    #[test]
    fn search_pattern_escapes_like_wildcards() {
        assert_eq!(
            search_pattern(&Some(" 50%_off ".to_string())),
            Some("%50\\%\\_off%".to_string())
        );
        assert_eq!(
            search_pattern(&Some("caja".to_string())),
            Some("%caja%".to_string())
        );
    }

    #[test]
    fn feed_message_appends_actor_only_when_present() {
        assert_eq!(
            feed_message("Venta #8 creada", Some("Ana")),
            "Venta #8 creada (Ana)"
        );
        assert_eq!(feed_message("Venta #8 creada", None), "Venta #8 creada");
        assert_eq!(
            feed_message("Venta #8 creada", Some("  ")),
            "Venta #8 creada"
        );
    }
}

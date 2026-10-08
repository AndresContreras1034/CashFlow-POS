use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{dto::AuditEventFilterDto, models::AuditEvent, repository};
use crate::modules::inventory::dto::PaginatedResponse;

fn validate_filter(filter: &AuditEventFilterDto) -> Result<(), AppError> {
    if let (Some(from), Some(to)) = (filter.date_from, filter.date_to) {
        if from > to {
            return Err(AppError::validation(
                "La fecha inicial no puede ser posterior a la final",
            ));
        }
    }
    Ok(())
}

pub async fn list_events(
    pool: &PgPool,
    filter: AuditEventFilterDto,
) -> Result<PaginatedResponse<AuditEvent>, AppError> {
    validate_filter(&filter)?;

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);

    let (data, total) = repository::list_events(pool, &filter)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn get_event(pool: &PgPool, id: i64) -> Result<AuditEvent, AppError> {
    repository::get_event(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Evento de auditoría no encontrado"))
}

/// "Ver operación completa": hasta 1000 eventos, del más antiguo al más reciente.
pub async fn list_by_correlation(
    pool: &PgPool,
    correlation_id: Uuid,
) -> Result<Vec<AuditEvent>, AppError> {
    repository::list_by_correlation(pool, correlation_id)
        .await
        .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn day(day: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(2026, 10, day)
    }

    #[test]
    fn inverted_date_range_is_rejected() {
        let filter = AuditEventFilterDto {
            date_from: day(10),
            date_to: day(5),
            ..Default::default()
        };
        assert!(validate_filter(&filter).is_err());
    }

    #[test]
    fn open_or_ordered_ranges_are_accepted() {
        assert!(validate_filter(&AuditEventFilterDto::default()).is_ok());

        let same_day = AuditEventFilterDto {
            date_from: day(5),
            date_to: day(5),
            ..Default::default()
        };
        assert!(validate_filter(&same_day).is_ok());

        let only_from = AuditEventFilterDto {
            date_from: day(5),
            ..Default::default()
        };
        assert!(validate_filter(&only_from).is_ok());
    }
}

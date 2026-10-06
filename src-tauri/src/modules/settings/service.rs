use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::settings::{dto::UpdateSettingsDto, models::AppSettings, repository};

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub async fn get_settings(pool: &PgPool) -> Result<AppSettings, AppError> {
    repository::get_settings(pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::internal("Ajustes no inicializados. Revisa la migración 004."))
}

pub async fn update_settings(
    pool: &PgPool,
    mut dto: UpdateSettingsDto,
) -> Result<AppSettings, AppError> {
    if let Some(ref name) = dto.business_name {
        if name.trim().is_empty() {
            return Err(AppError::validation(
                "El nombre del negocio no puede estar vacío",
            ));
        }
    }

    if let Some(bps) = dto.tax_rate_bps {
        if !(0..=10000).contains(&bps) {
            return Err(AppError::validation("El IVA debe estar entre 0% y 100%"));
        }
    }

    if let Some(ref currency) = dto.currency {
        let currency = currency.trim();
        if currency.chars().count() != 3
            || !currency
                .chars()
                .all(|character| character.is_ascii_alphabetic())
        {
            return Err(AppError::validation(
                "La moneda debe ser un código de 3 letras (ej: COP, USD)",
            ));
        }
    }

    if let Some(threshold) = dto.low_stock_default_threshold {
        if threshold < 0 {
            return Err(AppError::validation(
                "El umbral de stock bajo no puede ser negativo",
            ));
        }
    }

    dto.business_name = dto.business_name.map(|name| name.trim().to_string());
    dto.currency = dto.currency.map(|currency| currency.trim().to_uppercase());
    dto.tax_id = clean(dto.tax_id);
    dto.address = clean(dto.address);
    dto.phone = clean(dto.phone);
    dto.email = clean(dto.email);
    dto.ticket_header = clean(dto.ticket_header);
    dto.ticket_footer = clean(dto.ticket_footer);
    dto.logo_url = clean(dto.logo_url);

    repository::update_settings(pool, dto)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::internal("Ajustes no inicializados. Revisa la migración 004."))
}

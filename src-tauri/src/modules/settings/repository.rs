use sqlx::PgPool;

use crate::modules::settings::{dto::UpdateSettingsDto, models::AppSettings};

pub async fn get_settings(pool: &PgPool) -> Result<Option<AppSettings>, sqlx::Error> {
    sqlx::query_as!(AppSettings, "SELECT * FROM app_settings WHERE id = 1")
        .fetch_optional(pool)
        .await
}

pub async fn update_settings(
    pool: &PgPool,
    dto: UpdateSettingsDto,
) -> Result<Option<AppSettings>, sqlx::Error> {
    sqlx::query_as!(
        AppSettings,
        "UPDATE app_settings
         SET business_name               = COALESCE($1, business_name),
             tax_id                      = $2,
             address                     = $3,
             phone                       = $4,
             email                       = $5,
             currency                    = COALESCE($6, currency),
             tax_rate_bps                = COALESCE($7, tax_rate_bps),
             ticket_header               = $8,
             ticket_footer               = $9,
             low_stock_default_threshold = COALESCE($10, low_stock_default_threshold),
             logo_url                    = $11
         WHERE id = 1
         RETURNING *",
        dto.business_name,
        dto.tax_id,
        dto.address,
        dto.phone,
        dto.email,
        dto.currency,
        dto.tax_rate_bps,
        dto.ticket_header,
        dto.ticket_footer,
        dto.low_stock_default_threshold,
        dto.logo_url
    )
    .fetch_optional(pool)
    .await
}

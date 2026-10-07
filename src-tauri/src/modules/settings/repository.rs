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
             logo_url                    = $11,
             currency_decimals           = COALESCE($12, currency_decimals),
             tax_name                    = COALESCE($13, tax_name),
             timezone                    = COALESCE($14, timezone),
             show_logo                   = COALESCE($15, show_logo),
             show_tax_id                 = COALESCE($16, show_tax_id),
             show_address                = COALESCE($17, show_address),
             show_phone                  = COALESCE($18, show_phone),
             show_cashier                = COALESCE($19, show_cashier),
             show_tax_breakdown          = COALESCE($20, show_tax_breakdown),
             show_discounts              = COALESCE($21, show_discounts),
             show_payment_method         = COALESCE($22, show_payment_method)
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
        dto.logo_url,
        dto.currency_decimals,
        dto.tax_name,
        dto.timezone,
        dto.show_logo,
        dto.show_tax_id,
        dto.show_address,
        dto.show_phone,
        dto.show_cashier,
        dto.show_tax_breakdown,
        dto.show_discounts,
        dto.show_payment_method
    )
    .fetch_optional(pool)
    .await
}

/// Validate against PostgreSQL, which also uses the timezone for day grouping.
pub async fn timezone_exists(pool: &PgPool, name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM pg_timezone_names WHERE name = $1) AS "exists!""#,
        name
    )
    .fetch_one(pool)
    .await
}

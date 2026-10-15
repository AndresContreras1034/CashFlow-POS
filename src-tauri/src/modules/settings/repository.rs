use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::audit::{
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::settings::{dto::UpdateSettingsDto, models::AppSettings};

pub async fn get_settings(pool: &PgPool) -> Result<Option<AppSettings>, sqlx::Error> {
    sqlx::query_as::<_, AppSettings>("SELECT * FROM app_settings WHERE id = 1")
        .fetch_optional(pool)
        .await
}

pub async fn update_settings(
    pool: &PgPool,
    dto: UpdateSettingsDto,
) -> Result<Option<AppSettings>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let before =
        sqlx::query_as::<_, AppSettings>("SELECT * FROM app_settings WHERE id = 1 FOR UPDATE")
            .fetch_optional(&mut *tx)
            .await?;

    let Some(before) = before else {
        return Ok(None);
    };

    let after = sqlx::query_as::<_, AppSettings>(
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
             show_payment_method         = COALESCE($22, show_payment_method),
             printer_name                = COALESCE($23, printer_name)
         WHERE id = 1
         RETURNING *",
    )
    .bind(dto.business_name)
    .bind(dto.tax_id)
    .bind(dto.address)
    .bind(dto.phone)
    .bind(dto.email)
    .bind(dto.currency)
    .bind(dto.tax_rate_bps)
    .bind(dto.ticket_header)
    .bind(dto.ticket_footer)
    .bind(dto.low_stock_default_threshold)
    .bind(dto.logo_url)
    .bind(dto.currency_decimals)
    .bind(dto.tax_name)
    .bind(dto.timezone)
    .bind(dto.show_logo)
    .bind(dto.show_tax_id)
    .bind(dto.show_address)
    .bind(dto.show_phone)
    .bind(dto.show_cashier)
    .bind(dto.show_tax_breakdown)
    .bind(dto.show_discounts)
    .bind(dto.show_payment_method)
    .bind(dto.printer_name)
    .fetch_one(&mut *tx)
    .await?;

    let changes = changed_fields(&before, &after, &["id", "created_at", "updated_at"]);
    if !changes.is_empty() {
        let fields = changes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");

        audit_repo::insert_event(
            &mut tx,
            NewAuditEvent {
                correlation_id: Some(Uuid::new_v4()),
                category: AuditCategory::Business,
                module: AuditModule::Settings,
                action: "update".to_string(),
                outcome: AuditOutcome::Success,
                actor: None,
                entity_type: Some("app_settings".to_string()),
                entity_id: Some("1".to_string()),
                summary: format!("Ajustes actualizados: {}", fields),
                changes: Some(serde_json::Value::Object(changes)),
                metadata: None,
                error_message: None,
            },
        )
        .await?;
    }

    tx.commit().await?;
    Ok(Some(after))
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

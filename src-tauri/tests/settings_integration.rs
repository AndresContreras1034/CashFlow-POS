use pos_lib::modules::settings::{dto::UpdateSettingsDto, repository, service};
use sqlx::PgPool;

fn empty_update_settings(timezone: Option<&str>) -> UpdateSettingsDto {
    UpdateSettingsDto {
        business_name: None,
        tax_id: None,
        address: None,
        phone: None,
        email: None,
        currency: None,
        currency_decimals: None,
        tax_rate_bps: None,
        tax_name: None,
        timezone: timezone.map(str::to_string),
        ticket_header: None,
        ticket_footer: None,
        show_logo: None,
        show_tax_id: None,
        show_address: None,
        show_phone: None,
        show_cashier: None,
        show_tax_breakdown: None,
        show_discounts: None,
        show_payment_method: None,
        low_stock_default_threshold: None,
        logo_url: None,
        printer_name: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn saving_settings_without_printer_does_not_clear_printer_name(pool: PgPool) {
    let before = repository::get_settings(&pool)
        .await
        .unwrap()
        .expect("settings row is seeded by migrations");
    assert_eq!(before.printer_name.as_deref(), Some("DIG58IIA"));

    repository::update_settings(&pool, empty_update_settings(None))
        .await
        .unwrap();

    let after = repository::get_settings(&pool)
        .await
        .unwrap()
        .expect("settings row remains available");
    assert_eq!(after.printer_name, before.printer_name);
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_timezone_is_rejected_by_settings_service(pool: PgPool) {
    let before = repository::get_settings(&pool).await.unwrap().unwrap();
    let error = service::update_settings(&pool, empty_update_settings(Some("Not/A_Timezone")))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Zona horaria"));

    let after = repository::get_settings(&pool).await.unwrap().unwrap();
    assert_eq!(after.timezone, before.timezone);
}

#[sqlx::test(migrations = "./migrations")]
async fn debt_today_falls_back_to_utc_for_an_invalid_stored_timezone(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'Not/A_Timezone' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let uses_utc: bool =
        sqlx::query_scalar("SELECT debt_today() = (NOW() AT TIME ZONE 'UTC')::DATE")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(uses_utc);
}

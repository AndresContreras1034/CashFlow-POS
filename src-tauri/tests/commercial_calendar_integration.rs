use chrono::NaiveDate;
use pos_lib::modules::commercial_calendar::{dto::CommercialEventInput, service};
use sqlx::PgPool;

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid test date")
}

fn event_input(
    name: &str,
    rule: &str,
    event_date: Option<NaiveDate>,
    month: Option<i16>,
    day: Option<i16>,
    weekday: Option<i16>,
    nth: Option<i16>,
) -> CommercialEventInput {
    CommercialEventInput {
        name: name.to_string(),
        kind: "special".to_string(),
        rule: rule.to_string(),
        event_date,
        month,
        day,
        weekday,
        nth,
        prep_days: 7,
        is_active: true,
        actor: Some("calendar-test".to_string()),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn event_crud_is_validated_and_audited(pool: PgPool) {
    let seeded = service::list_events(&pool).await.unwrap();
    assert_eq!(seeded.len(), 2);
    assert!(seeded.iter().any(|event| event.day == Some(15)));
    assert!(seeded.iter().any(|event| event.day == Some(-1)));

    let created = service::create_event(
        &pool,
        event_input(
            "Día especial",
            "once",
            Some(date("2026-10-20")),
            None,
            None,
            None,
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(created.name, "Día especial");

    let updated = service::update_event(
        &pool,
        created.id,
        event_input(
            "Día especial editado",
            "monthly",
            None,
            None,
            Some(20),
            None,
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(updated.rule, "monthly");

    let invalid_rule = service::create_event(
        &pool,
        event_input("Inválido", "yearly", None, Some(13), Some(15), None, None),
    )
    .await;
    assert!(invalid_rule.is_err());
    let invalid_prep = service::create_event(
        &pool,
        CommercialEventInput {
            prep_days: 61,
            ..event_input("Inválido", "monthly", None, None, Some(1), None, None)
        },
    )
    .await;
    assert!(invalid_prep.is_err());

    service::delete_event(&pool, created.id, Some("calendar-test".to_string()))
        .await
        .unwrap();
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE entity_type = 'commercial_event' AND entity_id = $1",
    )
    .bind(created.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 3);
}

async fn insert_daily_sales(pool: &PgPool, from: NaiveDate, to: NaiveDate) {
    sqlx::query(
        r#"INSERT INTO sales (subtotal, total, status, created_by, created_at, gross_semantics)
           SELECT CASE WHEN dates.day >= DATE '2026-10-11' THEN 8_000 ELSE 10_000 END,
                  CASE WHEN dates.day >= DATE '2026-10-11' THEN 8_000 ELSE 10_000 END,
                  'completed', 'calendar-test',
                  dates.day::TIMESTAMP AT TIME ZONE 'UTC', TRUE
           FROM generate_series($1::DATE, $2::DATE, INTERVAL '1 day') AS dates(day)
           WHERE dates.day NOT IN (DATE '2026-09-24')"#,
    )
    .bind(from)
    .bind(to)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn pace_reports_insufficient_baseline_for_short_history(pool: PgPool) {
    let as_of = date("2026-10-15");
    insert_daily_sales(&pool, date("2026-10-01"), as_of).await;
    let pace = service::sales_pace_for_date(&pool, as_of, "UTC", 5)
        .await
        .unwrap();
    assert_eq!(pace.status, "insufficient");
    assert_eq!(pace.vs_baseline_bps, None);
    assert_eq!(pace.year_ago_date, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn pace_uses_four_weekday_samples_and_prior_year_when_history_is_long(pool: PgPool) {
    let as_of = date("2026-10-15");
    insert_daily_sales(&pool, date("2025-06-01"), as_of).await;
    let pace = service::sales_pace_for_date(&pool, as_of, "UTC", 5)
        .await
        .unwrap();
    assert_eq!(pace.status, "ok");
    assert_eq!(pace.actual_cents, 40_000);
    assert_eq!(pace.baseline_cents, Some(50_000));
    assert_eq!(pace.vs_baseline_bps, Some(-2_000));
    assert_eq!(pace.year_ago_date, Some(date("2025-10-16")));
}

#[sqlx::test(migrations = "./migrations")]
async fn pace_excludes_event_days_and_days_without_paid_sales_from_baseline(pool: PgPool) {
    let as_of = date("2026-10-15");
    insert_daily_sales(&pool, date("2026-09-17"), as_of).await;
    sqlx::query(
        "INSERT INTO cash_sessions (opening_amount, opened_by, opened_at) VALUES (0, 'calendar-test', '2026-09-24T12:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    service::create_event(
        &pool,
        event_input(
            "Evento histórico",
            "once",
            Some(date("2026-09-17")),
            None,
            None,
            None,
            None,
        ),
    )
    .await
    .unwrap();

    let pace = service::sales_pace_for_date(&pool, as_of, "UTC", 1)
        .await
        .unwrap();
    assert_eq!(pace.status, "insufficient");
    assert_eq!(pace.baseline_cents, None);
}

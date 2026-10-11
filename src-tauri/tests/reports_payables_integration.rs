use chrono::{Duration, NaiveDate};
use pos_lib::modules::{
    debts::{
        dto::DebtFilterDto,
        models::{DebtCategory, DebtStatus, DebtUrgency},
        repository,
    },
    reports::{
        dto::{AgingLabel, ReportFilter},
        service,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

async fn today(pool: &PgPool) -> NaiveDate {
    sqlx::query_scalar("SELECT debt_today()")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn add_debt(
    pool: &PgPool,
    creditor: &str,
    amount: i64,
    issued_on: NaiveDate,
    due_on: Option<NaiveDate>,
    category: DebtCategory,
    voided: bool,
) -> i32 {
    let id: i32 = sqlx::query_scalar(
        r#"INSERT INTO debts
               (creditor_name, category, nature, concept, original_amount, issued_on, due_on,
                idempotency_key, voided_at, void_reason)
           VALUES ($1, $2, 'operating', $3, $4, $5, $6, $7,
                   CASE WHEN $8 THEN NOW() END,
                   CASE WHEN $8 THEN 'test void' END)
           RETURNING id"#,
    )
    .bind(creditor)
    .bind(category)
    .bind(format!("concept-{creditor}-{amount}"))
    .bind(amount)
    .bind(issued_on)
    .bind(due_on)
    .bind(Uuid::new_v4())
    .bind(voided)
    .fetch_one(pool)
    .await
    .unwrap();
    id
}

async fn add_payment(pool: &PgPool, debt_id: i32, amount: i64, paid_on: NaiveDate, voided: bool) {
    let payment_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO debt_payments
               (debt_id, amount, paid_on, method, idempotency_key)
           VALUES ($1, $2, $3, 'cash', $4)
           RETURNING id"#,
    )
    .bind(debt_id)
    .bind(amount)
    .bind(paid_on)
    .bind(Uuid::new_v4())
    .fetch_one(pool)
    .await
    .unwrap();
    if voided {
        sqlx::query(
            "UPDATE debt_payments SET voided_at = NOW(), void_reason = 'test reversal' WHERE id = $1",
        )
        .bind(payment_id)
        .execute(pool)
        .await
        .unwrap();
    }
}

fn report_filter(from: NaiveDate, to: NaiveDate) -> ReportFilter {
    ReportFilter {
        date_from: from,
        date_to: to,
        granularity: None,
        compare_previous: false,
        category_id: None,
        limit: None,
        rank_by: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn payables_summary_excludes_voided_debts_and_payments_and_reconciles_with_debts(
    pool: PgPool,
) {
    let current = today(&pool).await;
    let from = current - Duration::days(1);
    let pending = add_debt(
        &pool,
        "Pending",
        1_000,
        current,
        Some(current + Duration::days(10)),
        DebtCategory::Maintenance,
        false,
    )
    .await;
    add_payment(&pool, pending, 100, current, true).await;

    let partial = add_debt(
        &pool,
        "Partial",
        2_000,
        from - Duration::days(3),
        Some(current + Duration::days(3)),
        DebtCategory::BusinessOperation,
        false,
    )
    .await;
    add_payment(&pool, partial, 100, from - Duration::days(1), false).await;
    add_payment(&pool, partial, 200, current, false).await;
    add_payment(&pool, partial, 300, current + Duration::days(1), false).await;

    let overdue = add_debt(
        &pool,
        "Overdue",
        3_000,
        from - Duration::days(40),
        Some(current - Duration::days(1)),
        DebtCategory::Maintenance,
        false,
    )
    .await;
    add_payment(&pool, overdue, 100, current, false).await;

    let paid = add_debt(
        &pool,
        "Paid",
        4_000,
        current,
        Some(current + Duration::days(4)),
        DebtCategory::OtherObligations,
        false,
    )
    .await;
    add_payment(&pool, paid, 4_000, current, false).await;
    let voided = add_debt(
        &pool,
        "Voided",
        5_000,
        current,
        Some(current + Duration::days(2)),
        DebtCategory::FixedServices,
        true,
    )
    .await;

    let filter = report_filter(from, current);
    let report = service::get_payables_summary(&pool, filter).await.unwrap();
    assert_eq!(report.pending_total, 5_300);
    assert_eq!(report.overdue_total, 2_900);
    assert_eq!(report.due_soon_total, 1_400);
    assert_eq!(report.active_count, 3);
    assert_eq!(report.overdue_count, 1);
    assert_eq!(report.due_soon_count, 1);
    assert_eq!(report.paid_in_range, 4_300);
    assert_eq!(report.payments_in_range_count, 3);
    assert_eq!(report.issued_in_range, 5_000);
    for (id, status) in [
        (pending, "pending"),
        (partial, "partially_paid"),
        (overdue, "overdue"),
        (paid, "paid"),
        (voided, "voided"),
    ] {
        let actual: String =
            sqlx::query_scalar("SELECT status::TEXT FROM debt_overview WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(actual, status);
    }

    let summary = repository::summary(&pool, &DebtFilterDto::default())
        .await
        .unwrap();
    assert_eq!(report.pending_total, summary.pending_total);
    assert_eq!(report.overdue_total, summary.overdue_total);
    assert_eq!(report.due_soon_total, summary.due_soon_total);
    assert_eq!(report.active_count, summary.active_count);
    assert_eq!(report.overdue_count, summary.overdue_count);
    assert_eq!(report.due_soon_count, summary.due_soon_count);
    assert_eq!(
        report
            .by_urgency
            .iter()
            .map(|bucket| (bucket.urgency, bucket.count, bucket.balance_total))
            .collect::<Vec<_>>(),
        vec![
            (DebtUrgency::Ok, 1, 1_000),
            (DebtUrgency::Alert, 1, 1_400),
            (DebtUrgency::Overdue, 1, 2_900),
        ]
    );

    let maintenance = report
        .by_category
        .iter()
        .find(|entry| entry.category == DebtCategory::Maintenance)
        .unwrap();
    assert_eq!(maintenance.count, 2);
    assert_eq!(maintenance.balance_total, 3_900);
    assert_eq!(maintenance.overdue_total, 2_900);
    assert_eq!(maintenance.share_bps, 7_358);
}

#[sqlx::test(migrations = "./migrations")]
async fn payables_aging_has_six_ordered_buckets_with_exact_boundaries(pool: PgPool) {
    let current = today(&pool).await;
    for (offset, category) in [
        (Some(0), DebtCategory::Maintenance),
        (Some(-1), DebtCategory::BusinessOperation),
        (Some(-30), DebtCategory::FixedServices),
        (Some(-31), DebtCategory::OtherObligations),
        (Some(-60), DebtCategory::Maintenance),
        (Some(-61), DebtCategory::BusinessOperation),
        (Some(-90), DebtCategory::OtherObligations),
        (Some(-91), DebtCategory::FixedServices),
        (None, DebtCategory::OtherObligations),
    ] {
        let due = offset.map(|days| current + Duration::days(days));
        add_debt(
            &pool,
            "Aging",
            100,
            current - Duration::days(120),
            due,
            category,
            false,
        )
        .await;
    }
    let buckets = service::get_payables_aging(&pool, report_filter(current, current))
        .await
        .unwrap();
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| bucket.label)
            .collect::<Vec<_>>(),
        vec![
            AgingLabel::NotDue,
            AgingLabel::Overdue1to30,
            AgingLabel::Overdue31to60,
            AgingLabel::Overdue61to90,
            AgingLabel::Overdue90Plus,
            AgingLabel::NoDueDate,
        ]
    );
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| bucket.count)
            .collect::<Vec<_>>(),
        vec![1, 2, 2, 2, 1, 1]
    );
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| bucket.balance_total)
            .collect::<Vec<_>>(),
        vec![100, 200, 200, 200, 100, 100]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn top_creditors_group_exact_names_and_upcoming_dues_order_by_due_date(pool: PgPool) {
    let current = today(&pool).await;
    let issue = current - Duration::days(60);
    let same_first = add_debt(
        &pool,
        "Same creditor",
        2_000,
        issue,
        Some(current + Duration::days(8)),
        DebtCategory::Maintenance,
        false,
    )
    .await;
    let same_second = add_debt(
        &pool,
        "Same creditor",
        1_000,
        issue,
        Some(current + Duration::days(5)),
        DebtCategory::BusinessOperation,
        false,
    )
    .await;
    let higher_balance = add_debt(
        &pool,
        "Higher balance",
        4_000,
        issue,
        Some(current + Duration::days(2)),
        DebtCategory::FixedServices,
        false,
    )
    .await;
    let overdue_id = add_debt(
        &pool,
        "Oldest due",
        500,
        issue,
        Some(current - Duration::days(1)),
        DebtCategory::OtherObligations,
        false,
    )
    .await;
    let paid_id = add_debt(
        &pool,
        "Paid soon",
        900,
        issue,
        Some(current + Duration::days(1)),
        DebtCategory::Maintenance,
        false,
    )
    .await;
    add_payment(&pool, paid_id, 900, current, false).await;

    let mut filter = report_filter(current - Duration::days(1), current);
    filter.limit = Some(2);
    let creditors = service::get_top_creditors(&pool, filter.clone())
        .await
        .unwrap();
    assert_eq!(creditors.len(), 2);
    assert_eq!(creditors[0].creditor_name, "Higher balance");
    assert_eq!(creditors[0].balance_total, 4_000);
    let same = creditors
        .iter()
        .find(|creditor| creditor.creditor_name == "Same creditor")
        .unwrap();
    assert_eq!(same.active_count, 2);
    assert_eq!(same.balance_total, 3_000);
    assert_eq!(same.next_due_on, Some(current + Duration::days(5)));

    filter.category_id = Some(i32::MAX);
    filter.limit = Some(25);
    let dues = service::get_upcoming_dues(&pool, filter).await.unwrap();
    assert_eq!(dues.first().map(|due| due.debt_id), Some(overdue_id));
    assert_eq!(
        dues.iter().map(|due| due.debt_id).collect::<Vec<_>>(),
        vec![overdue_id, higher_balance, same_second, same_first]
    );
    assert!(dues.iter().all(|due| due.debt_id != paid_id));
    assert!(dues[0].days_until_due < 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn payables_due_today_uses_the_same_settings_timezone_as_debt_overview(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'Pacific/Pago_Pago' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let local_today = today(&pool).await;
    let debt_id = add_debt(
        &pool,
        "Timezone creditor",
        1_000,
        local_today,
        Some(local_today),
        DebtCategory::Maintenance,
        false,
    )
    .await;
    let overview = repository::get_overview(&pool, debt_id)
        .await
        .unwrap()
        .unwrap();
    let dues = service::get_upcoming_dues(&pool, report_filter(local_today, local_today))
        .await
        .unwrap();
    let due = dues.iter().find(|row| row.debt_id == debt_id).unwrap();
    assert_eq!(overview.today, local_today);
    assert_eq!(due.days_until_due, overview.days_until_due.unwrap());
    assert_eq!(due.urgency, DebtUrgency::DueToday);
    assert_eq!(overview.status, DebtStatus::Pending);
}

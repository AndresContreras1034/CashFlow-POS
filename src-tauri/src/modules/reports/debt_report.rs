use chrono::NaiveDate;
use sqlx::PgPool;

use crate::modules::{
    debts::models::{DebtCategory, DebtUrgency},
    reports::dto::{
        AgingBucket, AgingLabel, DateRange, PayablesByCategory, PayablesSummary, TopCreditor,
        UpcomingDue, UrgencyBucket,
    },
};

const ACTIVE: &str = "o.status IN ('pending', 'partially_paid', 'overdue')";
const DUE_SOON: &str = "o.urgency IN ('alert', 'due_today')";

#[derive(sqlx::FromRow)]
struct PayablesTotals {
    pending_total: i64,
    overdue_total: i64,
    due_soon_total: i64,
    active_count: i64,
    overdue_count: i64,
    due_soon_count: i64,
    paid_in_range: i64,
    payments_in_range_count: i64,
    issued_in_range: i64,
}

#[derive(sqlx::FromRow)]
struct CategoryTotals {
    category: DebtCategory,
    count: i64,
    balance_total: i64,
    overdue_total: i64,
    share_bps: i32,
}

#[derive(sqlx::FromRow)]
struct UrgencyTotals {
    urgency: DebtUrgency,
    count: i64,
    balance_total: i64,
}

#[derive(sqlx::FromRow)]
struct AgingTotals {
    bucket: i32,
    count: i64,
    balance_total: i64,
}

#[derive(sqlx::FromRow)]
struct CreditorTotals {
    creditor_name: String,
    active_count: i64,
    balance_total: i64,
    overdue_total: i64,
    next_due_on: Option<NaiveDate>,
}

#[derive(sqlx::FromRow)]
struct UpcomingDueRow {
    debt_id: i32,
    creditor_name: String,
    concept: String,
    category: DebtCategory,
    due_on: NaiveDate,
    balance: i64,
    days_until_due: i32,
    urgency: DebtUrgency,
}

pub async fn fetch_payables_summary(
    pool: &PgPool,
    range: &DateRange,
) -> Result<PayablesSummary, sqlx::Error> {
    let totals = sqlx::query_as::<_, PayablesTotals>(&format!(
        r#"SELECT
               COALESCE(SUM(o.balance) FILTER (WHERE {ACTIVE}), 0)::BIGINT AS pending_total,
               COALESCE(SUM(o.balance) FILTER (WHERE o.status = 'overdue'), 0)::BIGINT
                   AS overdue_total,
               COALESCE(SUM(o.balance) FILTER (WHERE {DUE_SOON}), 0)::BIGINT
                   AS due_soon_total,
               COUNT(*) FILTER (WHERE {ACTIVE})::BIGINT AS active_count,
               COUNT(*) FILTER (WHERE o.status = 'overdue')::BIGINT AS overdue_count,
               COUNT(*) FILTER (WHERE {DUE_SOON})::BIGINT AS due_soon_count,
               (SELECT COALESCE(SUM(p.amount), 0)::BIGINT
                FROM debt_payments p
                JOIN debts d ON d.id = p.debt_id
                WHERE p.voided_at IS NULL AND d.voided_at IS NULL
                  AND p.paid_on >= $1 AND p.paid_on <= $2) AS paid_in_range,
               (SELECT COUNT(*)::BIGINT
                FROM debt_payments p
                JOIN debts d ON d.id = p.debt_id
                WHERE p.voided_at IS NULL AND d.voided_at IS NULL
                  AND p.paid_on >= $1 AND p.paid_on <= $2) AS payments_in_range_count,
               (SELECT COALESCE(SUM(d.original_amount), 0)::BIGINT
                FROM debts d
                WHERE d.voided_at IS NULL
                  AND d.issued_on >= $1 AND d.issued_on <= $2) AS issued_in_range
           FROM debt_overview o"#
    ))
    .bind(range.from)
    .bind(range.to)
    .fetch_one(pool)
    .await?;

    let by_category = sqlx::query_as::<_, CategoryTotals>(
        r#"SELECT
               o.category AS category,
               COUNT(*)::BIGINT AS count,
               SUM(o.balance)::BIGINT AS balance_total,
               COALESCE(SUM(o.balance) FILTER (WHERE o.status = 'overdue'), 0)::BIGINT
                   AS overdue_total,
               CASE WHEN SUM(SUM(o.balance)) OVER () = 0 THEN 0
                    ELSE ROUND(SUM(o.balance)::NUMERIC * 10000
                               / SUM(SUM(o.balance)) OVER ())::INT
               END AS share_bps
           FROM debt_overview o
           WHERE o.status IN ('pending', 'partially_paid', 'overdue')
           GROUP BY o.category
           ORDER BY o.category"#,
    )
    .fetch_all(pool)
    .await?;

    let by_urgency = sqlx::query_as::<_, UrgencyTotals>(
        r#"SELECT o.urgency, COUNT(*)::BIGINT AS count, SUM(o.balance)::BIGINT AS balance_total
           FROM debt_overview o
           WHERE o.status IN ('pending', 'partially_paid', 'overdue')
           GROUP BY o.urgency
           ORDER BY o.urgency"#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| UrgencyBucket {
        urgency: row.urgency,
        count: row.count,
        balance_total: row.balance_total,
    })
    .collect();

    Ok(PayablesSummary {
        pending_total: totals.pending_total,
        overdue_total: totals.overdue_total,
        due_soon_total: totals.due_soon_total,
        active_count: totals.active_count,
        overdue_count: totals.overdue_count,
        due_soon_count: totals.due_soon_count,
        paid_in_range: totals.paid_in_range,
        payments_in_range_count: totals.payments_in_range_count,
        issued_in_range: totals.issued_in_range,
        by_category: by_category
            .into_iter()
            .map(|row| PayablesByCategory {
                category: row.category,
                count: row.count,
                balance_total: row.balance_total,
                overdue_total: row.overdue_total,
                share_bps: row.share_bps,
            })
            .collect(),
        by_urgency,
    })
}

pub async fn fetch_payables_aging(pool: &PgPool) -> Result<Vec<AgingBucket>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AgingTotals>(
        r#"WITH bucket_labels(bucket) AS (
               VALUES (0), (1), (2), (3), (4), (5)
           ),
           classified AS (
               SELECT CASE
                   WHEN o.due_on IS NULL THEN 5
                   WHEN o.days_until_due >= 0 THEN 0
                   WHEN o.days_until_due >= -30 THEN 1
                   WHEN o.days_until_due >= -60 THEN 2
                   WHEN o.days_until_due >= -90 THEN 3
                   ELSE 4
               END AS bucket,
               o.balance
               FROM debt_overview o
               WHERE o.status IN ('pending', 'partially_paid', 'overdue')
                 AND o.balance > 0
           ),
           totals AS (
               SELECT bucket, COUNT(*)::BIGINT AS count, SUM(balance)::BIGINT AS balance_total
               FROM classified
               GROUP BY bucket
           )
           SELECT labels.bucket, COALESCE(totals.count, 0)::BIGINT AS count,
                  COALESCE(totals.balance_total, 0)::BIGINT AS balance_total
           FROM bucket_labels labels
           LEFT JOIN totals USING (bucket)
           ORDER BY labels.bucket"#,
    )
    .fetch_all(pool)
    .await?;

    let mut buckets = [
        AgingBucket {
            label: AgingLabel::NotDue,
            count: 0,
            balance_total: 0,
        },
        AgingBucket {
            label: AgingLabel::Overdue1to30,
            count: 0,
            balance_total: 0,
        },
        AgingBucket {
            label: AgingLabel::Overdue31to60,
            count: 0,
            balance_total: 0,
        },
        AgingBucket {
            label: AgingLabel::Overdue61to90,
            count: 0,
            balance_total: 0,
        },
        AgingBucket {
            label: AgingLabel::Overdue90Plus,
            count: 0,
            balance_total: 0,
        },
        AgingBucket {
            label: AgingLabel::NoDueDate,
            count: 0,
            balance_total: 0,
        },
    ];
    for row in rows {
        let bucket = buckets
            .get_mut(usize::try_from(row.bucket).map_err(|error| {
                sqlx::Error::Protocol(format!("Invalid debt aging bucket: {error}"))
            })?)
            .ok_or_else(|| sqlx::Error::Protocol("Invalid debt aging bucket".to_string()))?;
        bucket.count = row.count;
        bucket.balance_total = row.balance_total;
    }
    Ok(buckets.into())
}

pub async fn fetch_top_creditors(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<TopCreditor>, sqlx::Error> {
    Ok(sqlx::query_as::<_, CreditorTotals>(
        r#"SELECT
               o.creditor_name,
               COUNT(*)::BIGINT AS active_count,
               SUM(o.balance)::BIGINT AS balance_total,
               COALESCE(SUM(o.balance) FILTER (WHERE o.status = 'overdue'), 0)::BIGINT
                   AS overdue_total,
               MIN(o.due_on) FILTER (WHERE o.due_on >= o.today) AS next_due_on
           FROM debt_overview o
           WHERE o.status IN ('pending', 'partially_paid', 'overdue') AND o.balance > 0
           GROUP BY o.creditor_name
           ORDER BY balance_total DESC, o.creditor_name ASC
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| TopCreditor {
        creditor_name: row.creditor_name,
        active_count: row.active_count,
        balance_total: row.balance_total,
        overdue_total: row.overdue_total,
        next_due_on: row.next_due_on,
    })
    .collect())
}

pub async fn fetch_upcoming_dues(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<UpcomingDue>, sqlx::Error> {
    Ok(sqlx::query_as::<_, UpcomingDueRow>(
        r#"SELECT o.id AS debt_id, o.creditor_name, o.concept, o.category,
                  o.due_on, o.balance, COALESCE(o.days_until_due, 0) AS days_until_due,
                  o.urgency
           FROM debt_overview o
           WHERE o.status IN ('pending', 'partially_paid', 'overdue')
             AND o.due_on IS NOT NULL AND o.balance > 0
           ORDER BY o.due_on ASC, o.id ASC
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| UpcomingDue {
        debt_id: row.debt_id,
        creditor_name: row.creditor_name,
        concept: row.concept,
        category: row.category,
        due_on: row.due_on,
        balance: row.balance,
        days_until_due: row.days_until_due,
        urgency: row.urgency,
    })
    .collect())
}

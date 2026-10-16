use chrono::{Datelike, Duration, NaiveDate};

use crate::{
    errors::app_error::AppError,
    modules::commercial_calendar::{
        dto::{CommercialEvent, CommercialEventInput, SalesPace, UpcomingCommercialEvent},
        repository,
    },
};

const MAX_EVENT_NAME_CHARS: usize = 100;
const MAX_HORIZON_DAYS: i64 = 3660;
const MAX_PACE_DAYS: u16 = 31;
const BASELINE_SAMPLE_COUNT: usize = 4;
const MIN_BASELINE_SAMPLE_COUNT: usize = 3;

pub async fn list_events(pool: &sqlx::PgPool) -> Result<Vec<CommercialEvent>, AppError> {
    repository::list_events(pool).await.map_err(AppError::from)
}

pub async fn create_event(
    pool: &sqlx::PgPool,
    input: CommercialEventInput,
) -> Result<CommercialEvent, AppError> {
    repository::create_event(pool, validate_event(input)?).await
}

pub async fn update_event(
    pool: &sqlx::PgPool,
    id: i64,
    input: CommercialEventInput,
) -> Result<CommercialEvent, AppError> {
    if id <= 0 {
        return Err(AppError::validation(
            "El identificador del evento no es válido",
        ));
    }
    repository::update_event(pool, id, validate_event(input)?).await
}

pub async fn delete_event(
    pool: &sqlx::PgPool,
    id: i64,
    actor: Option<String>,
) -> Result<(), AppError> {
    if id <= 0 {
        return Err(AppError::validation(
            "El identificador del evento no es válido",
        ));
    }
    let actor = actor
        .map(|actor| actor.trim().to_string())
        .filter(|actor| !actor.is_empty());
    if actor
        .as_ref()
        .is_some_and(|actor| actor.chars().count() > 100)
    {
        return Err(AppError::validation(
            "El nombre del operador no puede superar 100 caracteres",
        ));
    }
    repository::delete_event(pool, id, actor).await
}

fn validate_event(mut input: CommercialEventInput) -> Result<CommercialEventInput, AppError> {
    input.name = input.name.trim().to_string();
    if input.name.is_empty() || input.name.chars().count() > MAX_EVENT_NAME_CHARS {
        return Err(AppError::validation(
            "El nombre del evento debe tener entre 1 y 100 caracteres",
        ));
    }
    if !["payday", "bonus", "holiday", "special"].contains(&input.kind.as_str()) {
        return Err(AppError::validation("El tipo de evento no es válido"));
    }
    if !(0..=60).contains(&input.prep_days) {
        return Err(AppError::validation(
            "Los días de preparación deben estar entre 0 y 60",
        ));
    }
    let valid_rule = match input.rule.as_str() {
        "once" => {
            input.event_date.is_some()
                && input.month.is_none()
                && input.day.is_none()
                && input.weekday.is_none()
                && input.nth.is_none()
        }
        "yearly" => {
            input.event_date.is_none()
                && input.month.is_some_and(|month| (1..=12).contains(&month))
                && input
                    .day
                    .is_some_and(|day| (1..=31).contains(&day) || day == -1)
                && input.weekday.is_none()
                && input.nth.is_none()
        }
        "monthly" => {
            input.event_date.is_none()
                && input.month.is_none()
                && input
                    .day
                    .is_some_and(|day| (1..=31).contains(&day) || day == -1)
                && input.weekday.is_none()
                && input.nth.is_none()
        }
        "nth_weekday" => {
            input.event_date.is_none()
                && input.month.is_some_and(|month| (1..=12).contains(&month))
                && input.day.is_none()
                && input
                    .weekday
                    .is_some_and(|weekday| (0..=6).contains(&weekday))
                && input
                    .nth
                    .is_some_and(|nth| (1..=4).contains(&nth) || nth == -1)
        }
        _ => false,
    };
    if !valid_rule {
        return Err(AppError::validation(
            "Los campos del evento no coinciden con la regla seleccionada",
        ));
    }
    input.actor = input
        .actor
        .map(|actor| actor.trim().to_string())
        .filter(|actor| !actor.is_empty());
    if input
        .actor
        .as_ref()
        .is_some_and(|actor| actor.chars().count() > 100)
    {
        return Err(AppError::validation(
            "El nombre del operador no puede superar 100 caracteres",
        ));
    }
    Ok(input)
}

pub fn next_events(
    today: NaiveDate,
    events: &[CommercialEvent],
    horizon_days: i64,
) -> Result<Vec<UpcomingCommercialEvent>, AppError> {
    if !(0..=MAX_HORIZON_DAYS).contains(&horizon_days) {
        return Err(AppError::validation(
            "El horizonte debe estar entre 0 y 3660 días",
        ));
    }
    let end = today
        .checked_add_signed(Duration::days(horizon_days))
        .ok_or_else(|| AppError::validation("El horizonte está fuera de rango"))?;
    let mut upcoming = Vec::new();
    for event in events.iter().filter(|event| event.is_active) {
        let mut date = today;
        while date <= end {
            if event_occurs_on(date, event) {
                upcoming.push(UpcomingCommercialEvent {
                    id: event.id,
                    name: event.name.clone(),
                    kind: event.kind.clone(),
                    date,
                    in_days: (date - today).num_days(),
                    prep_days: event.prep_days,
                });
            }
            let Some(next) = date.succ_opt() else {
                break;
            };
            date = next;
        }
    }
    upcoming.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(upcoming)
}

pub fn event_occurs_on(date: NaiveDate, event: &CommercialEvent) -> bool {
    if !event.is_active {
        return false;
    }
    match event.rule.as_str() {
        "once" => event.event_date == Some(date),
        "yearly" => {
            event.month == Some(date.month() as i16)
                && event.day.is_some_and(|day| {
                    if day == -1 {
                        date.day() == days_in_month(date.year(), date.month())
                    } else {
                        date.day() == day as u32
                    }
                })
        }
        "monthly" => event.day.is_some_and(|day| {
            if day == -1 {
                date.day() == days_in_month(date.year(), date.month())
            } else {
                date.day() == day as u32
            }
        }),
        "nth_weekday" => {
            event.month == Some(date.month() as i16)
                && event.weekday == Some(date.weekday().num_days_from_monday() as i16)
                && event.nth.is_some_and(|nth| {
                    if nth == -1 {
                        date.day() + 7 > days_in_month(date.year(), date.month())
                    } else {
                        (date.day() - 1) / 7 + 1 == nth as u32
                    }
                })
        }
        _ => false,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn baseline_from_four(samples: &[Option<i64>]) -> Option<i64> {
    if samples.len() != BASELINE_SAMPLE_COUNT {
        return None;
    }
    let valid: Vec<i128> = samples
        .iter()
        .flatten()
        .map(|value| i128::from(*value))
        .collect();
    if valid.len() < MIN_BASELINE_SAMPLE_COUNT {
        return None;
    }
    let average = (valid.iter().sum::<i128>() + (valid.len() as i128 / 2)) / valid.len() as i128;
    i64::try_from(average).ok()
}

pub fn baseline_for_date(
    date: NaiveDate,
    sales_by_day: &std::collections::HashMap<NaiveDate, i64>,
    events: &[CommercialEvent],
) -> Option<i64> {
    let samples: Vec<Option<i64>> = (1..=4)
        .map(|week| {
            let sample_date = date - Duration::days(week * 7);
            if events
                .iter()
                .any(|event| event_occurs_on(sample_date, event))
            {
                None
            } else {
                sales_by_day.get(&sample_date).copied()
            }
        })
        .collect();
    baseline_from_four(&samples)
}

pub fn pace_bps(actual: &[i64], baselines: &[Option<i64>]) -> Option<i64> {
    if actual.len() != baselines.len() || baselines.iter().any(Option::is_none) {
        return None;
    }
    let baseline: i128 = baselines
        .iter()
        .flatten()
        .map(|value| i128::from(*value))
        .sum();
    if baseline <= 0 {
        return None;
    }
    let actual: i128 = actual.iter().map(|value| i128::from(*value)).sum();
    i64::try_from((actual - baseline) * 10_000 / baseline).ok()
}

pub fn prior_year_reference_date(
    as_of: NaiveDate,
    earliest_history_date: Option<NaiveDate>,
) -> Option<NaiveDate> {
    let reference = as_of.checked_sub_signed(Duration::days(364))?;
    let history_threshold = reference.checked_sub_signed(Duration::days(60))?;
    earliest_history_date
        .filter(|earliest| *earliest <= history_threshold)
        .map(|_| reference)
}

pub async fn upcoming_events(
    pool: &sqlx::PgPool,
    horizon_days: i64,
) -> Result<Vec<UpcomingCommercialEvent>, AppError> {
    let (today, _) = local_today_and_timezone(pool).await?;
    let events = repository::list_events(pool)
        .await
        .map_err(AppError::from)?;
    next_events(today, &events, horizon_days)
}

pub async fn current_sales_pace(pool: &sqlx::PgPool, days: u16) -> Result<SalesPace, AppError> {
    let (as_of, timezone) = local_today_and_timezone(pool).await?;
    sales_pace_for_date(pool, as_of, &timezone, days).await
}

pub async fn sales_pace_for_date(
    pool: &sqlx::PgPool,
    as_of: NaiveDate,
    timezone: &str,
    days: u16,
) -> Result<SalesPace, AppError> {
    if !(1..=MAX_PACE_DAYS).contains(&days) {
        return Err(AppError::validation(
            "El ritmo debe cubrir entre 1 y 31 días",
        ));
    }
    let actual_start = as_of
        .checked_sub_signed(Duration::days(i64::from(days) - 1))
        .ok_or_else(|| AppError::validation("El rango de ritmo está fuera de límites"))?;
    let history_start = actual_start
        .checked_sub_signed(Duration::days(28))
        .ok_or_else(|| AppError::validation("El rango histórico está fuera de límites"))?;
    let sales_by_day =
        repository::daily_sales_and_open_days(pool, history_start, as_of, timezone).await?;
    let events = repository::list_events(pool)
        .await
        .map_err(AppError::from)?;
    let mut actual = Vec::with_capacity(days as usize);
    let mut baselines = Vec::with_capacity(days as usize);
    for offset in 0..days {
        let date = actual_start + Duration::days(i64::from(offset));
        actual.push(*sales_by_day.get(&date).unwrap_or(&0));
        baselines.push(baseline_for_date(date, &sales_by_day, &events));
    }
    let baseline_cents = baselines
        .iter()
        .all(Option::is_some)
        .then(|| baselines.iter().flatten().sum::<i64>());
    let vs_baseline_bps = pace_bps(&actual, &baselines);
    let earliest_history_date = repository::earliest_paid_sale_date(pool, timezone).await?;

    Ok(SalesPace {
        as_of,
        days,
        actual_cents: actual.iter().sum(),
        baseline_cents,
        vs_baseline_bps,
        status: if vs_baseline_bps.is_some() {
            "ok".to_string()
        } else {
            "insufficient".to_string()
        },
        year_ago_date: prior_year_reference_date(as_of, earliest_history_date),
    })
}

async fn local_today_and_timezone(pool: &sqlx::PgPool) -> Result<(NaiveDate, String), AppError> {
    sqlx::query_as::<_, (NaiveDate, String)>(
        "SELECT (NOW() AT TIME ZONE timezone)::DATE, timezone FROM app_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::internal("No se encontró la configuración regional del negocio"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::NaiveDate;

    use super::*;

    fn event(
        id: i64,
        rule: &str,
        date: Option<NaiveDate>,
        month: Option<i16>,
        day: Option<i16>,
        weekday: Option<i16>,
        nth: Option<i16>,
    ) -> CommercialEvent {
        CommercialEvent {
            id,
            name: format!("Evento {id}"),
            kind: "special".to_string(),
            rule: rule.to_string(),
            event_date: date,
            month,
            day,
            weekday,
            nth,
            prep_days: 7,
            is_active: true,
        }
    }

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn finds_once_yearly_monthly_and_last_weekday_events() {
        let events = vec![
            event(1, "once", Some(date("2026-10-18")), None, None, None, None),
            event(2, "yearly", None, Some(10), Some(18), None, None),
            event(3, "monthly", None, None, Some(-1), None, None),
            event(4, "nth_weekday", None, Some(10), None, Some(5), Some(-1)),
        ];
        let actual = next_events(date("2026-10-15"), &events, 20).unwrap();
        assert_eq!(
            actual.iter().map(|event| event.date).collect::<Vec<_>>(),
            vec![
                date("2026-10-18"),
                date("2026-10-18"),
                date("2026-10-31"),
                date("2026-10-31")
            ]
        );
    }

    #[test]
    fn finds_events_across_year_boundary() {
        let events = vec![
            event(1, "once", Some(date("2027-01-02")), None, None, None, None),
            event(2, "yearly", None, Some(1), Some(1), None, None),
            event(3, "monthly", None, None, Some(-1), None, None),
        ];
        let actual = next_events(date("2026-12-30"), &events, 5).unwrap();
        assert_eq!(
            actual.iter().map(|event| event.date).collect::<Vec<_>>(),
            vec![date("2026-12-31"), date("2027-01-01"), date("2027-01-02")]
        );
    }

    #[test]
    fn yearly_february_29_only_occurs_in_leap_years() {
        let leap_day = event(1, "yearly", None, Some(2), Some(29), None, None);
        assert!(event_occurs_on(date("2028-02-29"), &leap_day));
        assert!(!event_occurs_on(date("2027-02-28"), &leap_day));
    }

    #[test]
    fn once_events_are_not_repeated_and_inactive_events_are_ignored() {
        let mut one_time = event(1, "once", Some(date("2026-10-17")), None, None, None, None);
        let mut inactive = event(2, "monthly", None, None, Some(17), None, None);
        inactive.is_active = false;
        let actual = next_events(date("2026-10-15"), &[one_time.clone(), inactive], 40).unwrap();
        assert_eq!(actual.len(), 1);
        one_time.event_date = Some(date("2026-10-18"));
        assert_eq!(
            next_events(date("2026-10-15"), &[one_time], 40)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn baseline_requires_three_of_four_valid_open_days() {
        assert_eq!(
            baseline_from_four(&[Some(100), Some(200), Some(300), None]),
            Some(200)
        );
        assert_eq!(
            baseline_from_four(&[Some(100), Some(200), None, None]),
            None
        );
        assert_eq!(baseline_from_four(&[Some(100), Some(200), Some(300)]), None);
    }

    #[test]
    fn baseline_excludes_event_days_and_days_without_sales() {
        let target = date("2026-10-15");
        let samples = [
            date("2026-10-08"),
            date("2026-10-01"),
            date("2026-09-24"),
            date("2026-09-17"),
        ];
        let mut sales: HashMap<_, _> = samples.into_iter().map(|day| (day, 10_000)).collect();
        sales.remove(&date("2026-09-24"));
        let commercial_event = event(1, "once", Some(date("2026-09-17")), None, None, None, None);
        assert_eq!(baseline_for_date(target, &sales, &[]), Some(10_000));
        assert_eq!(baseline_for_date(target, &sales, &[commercial_event]), None);
    }

    #[test]
    fn pace_is_relative_to_sum_of_daily_baselines() {
        assert_eq!(pace_bps(&[80, 80], &[Some(100), Some(100)]), Some(-2_000));
        assert_eq!(pace_bps(&[120], &[Some(100)]), Some(2_000));
        assert_eq!(pace_bps(&[80, 80], &[Some(100), None]), None);
        assert_eq!(pace_bps(&[80], &[Some(0)]), None);
    }

    #[test]
    fn prior_year_reference_requires_sixty_days_of_earlier_history() {
        let as_of = date("2026-10-15");
        let reference = date("2025-10-16");
        assert_eq!(
            prior_year_reference_date(as_of, Some(date("2025-08-17"))),
            Some(reference)
        );
        assert_eq!(
            prior_year_reference_date(as_of, Some(date("2025-08-18"))),
            None
        );
        assert_eq!(prior_year_reference_date(as_of, None), None);
    }
}

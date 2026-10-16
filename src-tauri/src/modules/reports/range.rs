use chrono::{Datelike, Duration, NaiveDate};

use crate::errors::app_error::AppError;

use super::dto::{DateRange, Granularity, ReportFilter, TimeSeriesPoint};

pub const MAX_RANGE_DAYS: i64 = 732;
const DAILY_LIMIT_DAYS: i64 = 92;

pub fn granularity_for(range_days: i64) -> Granularity {
    if range_days > DAILY_LIMIT_DAYS {
        Granularity::Month
    } else {
        Granularity::Day
    }
}

impl ReportFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.date_from > self.date_to {
            return Err(AppError::validation(
                "La fecha inicial no puede ser posterior a la fecha final",
            ));
        }

        let inclusive_days = (self.date_to - self.date_from).num_days() + 1;
        if inclusive_days > MAX_RANGE_DAYS {
            return Err(AppError::validation("El rango no puede superar 732 días"));
        }

        if self.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
            return Err(AppError::validation("El límite debe estar entre 1 y 100"));
        }

        Ok(())
    }
}

pub fn previous_range(range: &DateRange) -> DateRange {
    let days = (range.to - range.from).num_days() + 1;
    let to = range.from - Duration::days(1);
    let from = to - Duration::days(days - 1);
    DateRange { from, to }
}

pub fn previous_year_range(range: &DateRange) -> Option<DateRange> {
    let shift_year = |date: NaiveDate| {
        let year = date.year().checked_sub(1)?;
        date.with_year(year)
            .or_else(|| NaiveDate::from_ymd_opt(year, date.month(), date.day().saturating_sub(1)))
    };
    Some(DateRange {
        from: shift_year(range.from)?,
        to: shift_year(range.to)?,
    })
}

pub fn month_starts(from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let Some(mut current) = NaiveDate::from_ymd_opt(from.year(), from.month(), 1) else {
        return Vec::new();
    };
    let mut months = Vec::new();
    while current <= to {
        months.push(current);
        let (year, month) = if current.month() == 12 {
            (current.year().checked_add(1), 1)
        } else {
            (Some(current.year()), current.month() + 1)
        };
        let Some(next) = year.and_then(|year| NaiveDate::from_ymd_opt(year, month, 1)) else {
            break;
        };
        current = next;
    }
    months
}

pub fn change_bps(current: i64, previous: i64) -> Option<i32> {
    if previous == 0 {
        return None;
    }

    Some(ratio_bps(
        (current as i128 - previous as i128) * 10_000,
        previous as i128,
    ))
}

pub fn share_bps(part: i64, total: i64) -> i32 {
    if total == 0 {
        return 0;
    }

    ratio_bps(part as i128 * 10_000, total as i128)
}

pub fn avg_ticket(paid_total: i64, paid_tickets: i64) -> i64 {
    if paid_tickets <= 0 {
        return 0;
    }

    let total = i128::from(paid_total);
    let tickets = i128::from(paid_tickets);
    let magnitude = total.abs();
    let rounded = (magnitude + tickets / 2) / tickets;
    let signed = if total < 0 { -rounded } else { rounded };
    signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

pub fn margin_bps(profit: i64, revenue: i64) -> i32 {
    share_bps(profit, revenue)
}

pub fn fill_buckets(
    range: &DateRange,
    granularity: Granularity,
    points: Vec<TimeSeriesPoint>,
) -> Result<Vec<TimeSeriesPoint>, AppError> {
    let points_by_bucket = points
        .into_iter()
        .map(|mut point| {
            point.partial =
                granularity == Granularity::Month && is_partial_month(point.bucket, range);
            (point.bucket, point)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut bucket = bucket_start(range.from, granularity)?;
    let mut filled = Vec::new();

    while bucket <= range.to {
        filled.push(match points_by_bucket.get(&bucket) {
            Some(point) => point.clone(),
            None => TimeSeriesPoint {
                bucket,
                partial: granularity == Granularity::Month && is_partial_month(bucket, range),
                tickets: 0,
                gross_sales: 0,
                gross_profit: 0,
            },
        });

        let next_bucket = match granularity {
            Granularity::Day => bucket.checked_add_signed(Duration::days(1)),
            Granularity::Week => bucket.checked_add_signed(Duration::days(7)),
            Granularity::Month => {
                let (year, month) = if bucket.month() == 12 {
                    (bucket.year().checked_add(1), 1)
                } else {
                    (Some(bucket.year()), bucket.month() + 1)
                };
                year.and_then(|year| NaiveDate::from_ymd_opt(year, month, 1))
            }
        };
        let Some(next_bucket) = next_bucket else {
            break;
        };
        bucket = next_bucket;
    }

    Ok(filled)
}

fn is_partial_month(bucket: NaiveDate, range: &DateRange) -> bool {
    let starts_mid_month = bucket == month_start(range.from) && range.from.day() > 1;
    let ends_mid_month =
        bucket == month_start(range.to) && range.to.day() < days_in_month(range.to);
    starts_mid_month || ends_mid_month
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

fn days_in_month(date: NaiveDate) -> u32 {
    let (year, month) = if date.month() == 12 {
        (date.year().checked_add(1), 1)
    } else {
        (Some(date.year()), date.month() + 1)
    };
    year.and_then(|year| NaiveDate::from_ymd_opt(year, month, 1))
        .and_then(|first_next_month| first_next_month.pred_opt())
        .map_or(31, |date| date.day())
}

fn bucket_start(date: NaiveDate, granularity: Granularity) -> Result<NaiveDate, AppError> {
    match granularity {
        Granularity::Day => Ok(date),
        Granularity::Week => {
            let offset = i64::from(date.weekday().num_days_from_monday());
            date.checked_sub_signed(Duration::days(offset))
                .ok_or_else(|| AppError::validation("La fecha del rango está fuera de límites"))
        }
        Granularity::Month => NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
            .ok_or_else(|| AppError::validation("La fecha del rango está fuera de límites")),
    }
}

fn ratio_bps(numerator: i128, denominator: i128) -> i32 {
    let negative = (numerator < 0) != (denominator < 0);
    let numerator = numerator.abs();
    let denominator = denominator.abs();
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounded = quotient + i128::from(remainder * 2 >= denominator);
    let signed = if negative { -rounded } else { rounded };
    signed.clamp(i32::MIN as i128, i32::MAX as i128) as i32
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid test date")
    }

    fn filter(from: &str, to: &str) -> ReportFilter {
        ReportFilter {
            date_from: date(from),
            date_to: date(to),
            granularity: Some(Granularity::Day),
            compare_previous: false,
            compare_year_over_year: false,
            category_id: None,
            limit: None,
            rank_by: None,
        }
    }

    #[test]
    fn accepts_one_day_range() {
        assert!(filter("2026-10-10", "2026-10-10").validate().is_ok());
    }

    #[test]
    fn accepts_thirty_day_range() {
        assert!(filter("2026-10-01", "2026-10-30").validate().is_ok());
    }

    #[test]
    fn previous_range_preserves_exact_length_within_a_month_boundary() {
        let selected = DateRange {
            from: date("2026-10-01"),
            to: date("2026-10-15"),
        };
        assert_eq!(
            previous_range(&selected),
            DateRange {
                from: date("2026-09-16"),
                to: date("2026-09-30"),
            }
        );
    }

    #[test]
    fn accepts_exactly_366_inclusive_days() {
        assert!(filter("2025-01-01", "2026-01-01").validate().is_ok());
    }

    #[test]
    fn accepts_exactly_732_days_and_rejects_longer_ranges() {
        assert!(filter("2024-10-14", "2026-10-15").validate().is_ok());
        assert!(filter("2024-10-13", "2026-10-15").validate().is_err());
    }

    #[test]
    fn selects_daily_or_monthly_granularity_at_92_days() {
        assert_eq!(granularity_for(92), Granularity::Day);
        assert_eq!(granularity_for(93), Granularity::Month);
    }

    #[test]
    fn month_starts_include_partial_boundary_months_across_years() {
        assert_eq!(
            month_starts(date("2025-11-20"), date("2026-02-03")),
            vec![
                date("2025-11-01"),
                date("2025-12-01"),
                date("2026-01-01"),
                date("2026-02-01"),
            ]
        );
        assert_eq!(
            month_starts(date("2026-10-05"), date("2026-10-20")),
            vec![date("2026-10-01")]
        );
    }

    #[test]
    fn previous_year_range_clamps_leap_day_to_february_28() {
        assert_eq!(
            previous_year_range(&DateRange {
                from: date("2028-02-29"),
                to: date("2028-03-10"),
            }),
            Some(DateRange {
                from: date("2027-02-28"),
                to: date("2027-03-10"),
            })
        );
    }

    #[test]
    fn validates_ranges_across_year_boundary() {
        let selected = DateRange {
            from: date("2026-12-20"),
            to: date("2027-01-04"),
        };
        assert_eq!(
            previous_range(&selected),
            DateRange {
                from: date("2026-12-04"),
                to: date("2026-12-19"),
            }
        );
        assert!(filter("2026-12-20", "2027-01-04").validate().is_ok());
    }

    #[test]
    fn rejects_start_after_end() {
        assert!(filter("2026-10-11", "2026-10-10").validate().is_err());
    }

    #[test]
    fn rejects_ranges_longer_than_732_inclusive_days() {
        assert!(filter("2024-10-13", "2026-10-15").validate().is_err());
    }

    #[test]
    fn validates_ranking_limit() {
        assert!(filter("2026-10-10", "2026-10-10").validate().is_ok());
        let mut invalid = filter("2026-10-10", "2026-10-10");
        invalid.limit = Some(0);
        assert!(invalid.validate().is_err());
        invalid.limit = Some(101);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn change_is_none_for_zero_previous_and_supports_negative_values() {
        assert_eq!(change_bps(100, 0), None);
        assert_eq!(change_bps(50, 100), Some(-5_000));
        assert_eq!(change_bps(50, -100), Some(-15_000));
    }

    #[test]
    fn change_rounds_half_away_from_zero() {
        assert_eq!(change_bps(1, 20_000), Some(-10_000));
        assert_eq!(change_bps(20_001, 20_000), Some(1));
        assert_eq!(change_bps(19_999, 20_000), Some(-1));
    }

    #[test]
    fn change_saturates_to_i32() {
        assert_eq!(change_bps(i64::MAX, 1), Some(i32::MAX));
    }

    #[test]
    fn share_and_margin_handle_zero_total() {
        assert_eq!(share_bps(10, 0), 0);
        assert_eq!(margin_bps(10, 0), 0);
        assert_eq!(share_bps(1, 8), 1_250);
        assert_eq!(margin_bps(2, 8), 2_500);
    }

    #[test]
    fn customer_share_is_based_on_total_sales_including_walk_ins() {
        assert_eq!(share_bps(3_000, 10_000), 3_000);
    }

    #[test]
    fn average_ticket_uses_paid_tickets_only() {
        assert_eq!(avg_ticket(30_000, 2), 15_000);
    }

    #[test]
    fn average_ticket_with_no_paid_tickets_is_zero() {
        assert_eq!(avg_ticket(5, 0), 0);
    }

    fn point(bucket: &str, tickets: i64) -> TimeSeriesPoint {
        TimeSeriesPoint {
            bucket: date(bucket),
            partial: false,
            tickets,
            gross_sales: tickets * 100,
            gross_profit: tickets * 50,
        }
    }

    #[test]
    fn monthly_buckets_flag_partial_start_and_end_months() {
        let range = DateRange {
            from: date("2025-11-20"),
            to: date("2026-02-03"),
        };
        let filled = fill_buckets(&range, Granularity::Month, Vec::new()).unwrap();
        assert_eq!(
            filled.iter().map(|point| point.bucket).collect::<Vec<_>>(),
            month_starts(range.from, range.to)
        );
        assert_eq!(
            filled.iter().map(|point| point.partial).collect::<Vec<_>>(),
            vec![true, false, false, true]
        );
    }

    #[test]
    fn fills_daily_buckets_with_zeroes_and_supports_a_one_day_range() {
        let range = DateRange {
            from: date("2026-10-10"),
            to: date("2026-10-12"),
        };
        let filled = fill_buckets(
            &range,
            Granularity::Day,
            vec![point("2026-10-10", 2), point("2026-10-12", 1)],
        )
        .expect("valid daily range");

        assert_eq!(filled.len(), 3);
        assert_eq!(filled[0].tickets, 2);
        assert_eq!(filled[1], point("2026-10-11", 0));
        assert_eq!(
            fill_buckets(
                &DateRange {
                    from: date("2026-10-10"),
                    to: date("2026-10-10"),
                },
                Granularity::Day,
                Vec::new(),
            )
            .expect("valid daily range"),
            vec![point("2026-10-10", 0)]
        );
    }

    #[test]
    fn fills_week_buckets_from_monday_across_a_month_boundary() {
        let filled = fill_buckets(
            &DateRange {
                from: date("2026-11-01"),
                to: date("2026-11-08"),
            },
            Granularity::Week,
            vec![point("2026-10-26", 2), point("2026-11-02", 3)],
        )
        .expect("valid week range");

        assert_eq!(
            filled.iter().map(|point| point.bucket).collect::<Vec<_>>(),
            vec![date("2026-10-26"), date("2026-11-02")]
        );
        assert_eq!(filled[0].tickets, 2);
        assert_eq!(filled[1].tickets, 3);
    }

    #[test]
    fn fills_month_buckets_across_a_year_boundary() {
        let filled = fill_buckets(
            &DateRange {
                from: date("2026-12-20"),
                to: date("2027-02-02"),
            },
            Granularity::Month,
            vec![point("2026-12-01", 1), point("2027-02-01", 2)],
        )
        .expect("valid month range");

        assert_eq!(
            filled.iter().map(|point| point.bucket).collect::<Vec<_>>(),
            vec![date("2026-12-01"), date("2027-01-01"), date("2027-02-01")]
        );
        assert_eq!(filled[1].tickets, 0);
    }
}

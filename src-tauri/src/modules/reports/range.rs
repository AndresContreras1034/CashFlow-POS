use chrono::Duration;

use crate::errors::app_error::AppError;

use super::dto::{DateRange, ReportFilter};

impl ReportFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.date_from > self.date_to {
            return Err(AppError::validation(
                "La fecha inicial no puede ser posterior a la fecha final",
            ));
        }

        let inclusive_days = (self.date_to - self.date_from).num_days() + 1;
        if inclusive_days > 366 {
            return Err(AppError::validation("El rango no puede superar 366 días"));
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

pub fn margin_bps(profit: i64, revenue: i64) -> i32 {
    share_bps(profit, revenue)
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
    use crate::modules::reports::dto::Granularity;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid test date")
    }

    fn filter(from: &str, to: &str) -> ReportFilter {
        ReportFilter {
            date_from: date(from),
            date_to: date(to),
            granularity: Some(Granularity::Day),
            compare_previous: false,
            category_id: None,
            limit: None,
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
    fn rejects_ranges_longer_than_366_inclusive_days() {
        assert!(filter("2025-01-01", "2026-01-02").validate().is_err());
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
}

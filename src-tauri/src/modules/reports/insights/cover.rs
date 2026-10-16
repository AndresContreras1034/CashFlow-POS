#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Cover {
    Days(i64),
    NoSales,
    Insufficient,
}

pub const MIN_HISTORY_DAYS: i64 = 28;

pub fn days_cover(stock: i64, units_sold: i64, window_days: i64, history_days: i64) -> Cover {
    if history_days < MIN_HISTORY_DAYS || window_days <= 0 {
        return Cover::Insufficient;
    }
    if units_sold <= 0 {
        return Cover::NoSales;
    }
    Cover::Days((stock.max(0) as i128 * window_days as i128 / units_sold as i128) as i64)
}

/// Riesgo si la cobertura no alcanza hasta el evento + margen de 3 días
pub fn shortage_risk(cover: Cover, days_to_event: i64) -> bool {
    match cover {
        Cover::Days(d) => d < days_to_event + 3,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal() {
        assert_eq!(days_cover(40, 28, 28, 90), Cover::Days(40));
    }

    #[test]
    fn half_rate() {
        assert_eq!(days_cover(10, 56, 28, 90), Cover::Days(5));
    }

    #[test]
    fn short_history() {
        assert_eq!(days_cover(10, 5, 28, 10), Cover::Insufficient);
    }

    #[test]
    fn no_sales() {
        assert_eq!(days_cover(10, 0, 28, 90), Cover::NoSales);
    }

    #[test]
    fn risk_flags() {
        assert!(shortage_risk(Cover::Days(4), 4));
        assert!(!shortage_risk(Cover::Days(7), 4));
        assert!(!shortage_risk(Cover::NoSales, 4));
        assert!(!shortage_risk(Cover::Insufficient, 4));
    }
}

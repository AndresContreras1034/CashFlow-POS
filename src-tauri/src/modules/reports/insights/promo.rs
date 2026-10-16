use serde::Serialize;

fn pct_of(amount: i64, pct: i64) -> i64 {
    ((amount as i128 * pct as i128 + 50) / 100) as i64
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Scenario {
    pub kind: &'static str,
    pub revenue: i64,
    pub cost: i64,
    pub profit: i64,
    pub margin_bps: i64,
    pub allowed: bool,
    /// Multiplicador x100 de transacciones necesarias para igualar la utilidad normal
    pub breakeven_x100: Option<i64>,
}

fn build(
    kind: &'static str,
    revenue: i64,
    cost: i64,
    normal_profit: i64,
    min_bps: i64,
) -> Scenario {
    let profit = revenue - cost;
    let margin_bps = if revenue > 0 {
        (profit as i128 * 10_000 / revenue as i128) as i64
    } else {
        i64::MIN
    };
    let allowed = profit > 0 && margin_bps >= min_bps;
    let breakeven_x100 = if profit > 0 && normal_profit > 0 {
        Some(((normal_profit as i128 * 100 + profit as i128 - 1) / profit as i128) as i64)
    } else {
        None
    };
    Scenario {
        kind,
        revenue,
        cost,
        profit,
        margin_bps,
        allowed,
        breakeven_x100,
    }
}

pub fn single_product(
    price: i64,
    cost: i64,
    min_bps: i64,
    second_off: i64,
    direct_off: i64,
) -> Vec<Scenario> {
    let normal = price - cost;
    vec![
        build("two_for_one", price, cost * 2, normal, min_bps),
        build(
            "second_unit_off",
            price + price - pct_of(price, second_off),
            cost * 2,
            normal,
            min_bps,
        ),
        build(
            "direct_discount",
            price - pct_of(price, direct_off),
            cost,
            normal,
            min_bps,
        ),
    ]
}

/// Combo: dos productos distintos con descuento sobre la suma
pub fn bundle(p1: i64, c1: i64, p2: i64, c2: i64, off_pct: i64, min_bps: i64) -> Scenario {
    let sum = p1 + p2;
    let normal = (p1 - c1) + (p2 - c2);
    build(
        "bundle",
        sum - pct_of(sum, off_pct),
        c1 + c2,
        normal,
        min_bps,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfume_6000_15000() {
        let s = single_product(15_000, 6_000, 1_500, 50, 10);
        assert_eq!(s[0].profit, 3_000);
        assert_eq!(s[0].breakeven_x100, Some(300));
        assert!(s[0].allowed); // 3000/15000 = 20 % >= 15 %
    }
    #[test]
    fn second_half_off_beats_normal() {
        let s = single_product(15_000, 6_000, 1_500, 50, 10);
        assert_eq!(s[1].profit, 10_500);
        assert_eq!(s[2].profit, 7_500);
    }
    #[test]
    fn never_below_cost() {
        let s = single_product(10_000, 6_000, 0, 50, 60);
        assert!(!s[2].allowed);
    }
    #[test]
    fn bundle_respects_margin() {
        let b = bundle(10_000, 7_000, 12_000, 9_000, 20, 1_500);
        assert_eq!(b.revenue, 17_600);
        assert_eq!(b.profit, 1_600);
        assert!(!b.allowed); // 1600/17600 = 9 % < 15 %
    }
}

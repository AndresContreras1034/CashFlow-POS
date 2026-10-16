use super::promo::{bundle, Scenario};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Item {
    pub variant_id: i64,
    pub category_id: i64,
    pub price: i64,
    pub cost: i64,        // 0 = sin costo
    pub stock_value: i64, // solo relevante en parados
}

#[derive(Debug, Serialize)]
pub struct ComboCandidate {
    pub stale_variant_id: i64,
    pub star_variant_id: i64,
    pub scenario: Scenario,
}

pub struct ComboResult {
    pub candidates: Vec<ComboCandidate>,
    pub excluded_no_cost: usize,
}

fn similar_price(a: i64, b: i64) -> bool {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    lo > 0 && hi * 100 <= lo * 130 // dentro de +30 %
}

pub fn pick_combos(stale: &[Item], stars: &[Item], off_pct: i64, min_bps: i64) -> ComboResult {
    let mut sorted: Vec<&Item> = stale.iter().collect();
    sorted.sort_by(|a, b| {
        b.stock_value
            .cmp(&a.stock_value)
            .then(a.variant_id.cmp(&b.variant_id))
    });
    let mut out = Vec::new();
    let mut excluded = 0usize;
    for s in sorted {
        if s.cost <= 0 {
            excluded += 1;
            continue;
        }
        if out.len() >= 3 {
            continue;
        }
        let star = stars.iter().find(|t| {
            t.cost > 0 && (t.category_id == s.category_id || similar_price(t.price, s.price))
        });
        if let Some(t) = star {
            let sc = bundle(s.price, s.cost, t.price, t.cost, off_pct, min_bps);
            if sc.allowed {
                out.push(ComboCandidate {
                    stale_variant_id: s.variant_id,
                    star_variant_id: t.variant_id,
                    scenario: sc,
                });
            }
        }
    }
    ComboResult {
        candidates: out,
        excluded_no_cost: excluded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(id: i64, cat: i64, price: i64, cost: i64, val: i64) -> Item {
        Item {
            variant_id: id,
            category_id: cat,
            price,
            cost,
            stock_value: val,
        }
    }

    #[test]
    fn max_three_sorted_by_value_and_counts_uncosted() {
        let stale = vec![
            it(1, 1, 10_000, 3_000, 500),
            it(2, 1, 10_000, 3_000, 900),
            it(3, 1, 10_000, 0, 800),
            it(4, 1, 10_000, 3_000, 100),
            it(5, 1, 10_000, 3_000, 700),
        ];
        let stars = vec![it(99, 1, 12_000, 4_000, 0)];
        let r = pick_combos(&stale, &stars, 10, 1_500);
        assert_eq!(r.candidates.len(), 3);
        assert_eq!(r.candidates[0].stale_variant_id, 2);
        assert_eq!(r.excluded_no_cost, 1);
    }

    #[test]
    fn rejects_when_margin_too_low() {
        let stale = vec![it(1, 1, 10_000, 9_000, 500)];
        let stars = vec![it(99, 1, 12_000, 10_000, 0)];
        assert!(pick_combos(&stale, &stars, 10, 1_500).candidates.is_empty());
    }
}

use super::advice::{Advice, AdviceResponse, Cited};
use super::promo::single_product;
use serde_json::Value;

const MAX_ADVICE: usize = 3;
const MAX_OFF_PCT: i64 = 90;

fn same_value(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < f64::EPSILON,
        _ => a == b,
    }
}

fn cited_ok(input: &Value, c: &Cited) -> bool {
    input
        .pointer(&c.path)
        .is_some_and(|actual| same_value(actual, &c.value))
}

fn promo_ok(input: &Value, a: &Advice, min_bps: i64) -> bool {
    let Some(p) = &a.promo else { return true };
    let second = p.second_off_pct.unwrap_or(0);
    let direct = p.direct_off_pct.unwrap_or(0);
    if !(0..=MAX_OFF_PCT).contains(&second) || !(0..=MAX_OFF_PCT).contains(&direct) {
        return false;
    }
    match p.kind.as_str() {
        "second_unit_off" if p.second_off_pct.is_none() => return false,
        "direct_discount" if p.direct_off_pct.is_none() => return false,
        "two_for_one" | "second_unit_off" | "direct_discount" => {}
        _ => return false,
    }
    let Some(item) = input
        .pointer("/promo_candidates")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|r| r.get("alias").and_then(Value::as_str) == Some(p.alias.as_str()))
        })
    else {
        return false;
    };
    let (Some(price), Some(cost)) = (
        item.get("price").and_then(Value::as_i64),
        item.get("cost").and_then(Value::as_i64),
    ) else {
        return false;
    };
    single_product(price, cost, min_bps, second, direct)
        .iter()
        .any(|s| s.kind == p.kind && s.allowed)
}

/// Descarta consejos sin cifras verificables o con promociones no permitidas.
pub fn validate(resp: AdviceResponse, input: &Value, min_bps: i64) -> AdviceResponse {
    if resp.insufficient_data {
        return AdviceResponse {
            insufficient_data: true,
            advice: Vec::new(),
        };
    }
    let advice: Vec<Advice> = resp
        .advice
        .into_iter()
        .filter(|a| !a.cited.is_empty() && a.cited.iter().all(|c| cited_ok(input, c)))
        .filter(|a| promo_ok(input, a, min_bps))
        .take(MAX_ADVICE)
        .collect();
    AdviceResponse {
        insufficient_data: false,
        advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> Value {
        json!({
            "pace": {"vs_baseline_bps": -1800},
            "promo_candidates": [{"alias": "Producto 2", "price": 15000, "cost": 6000}]
        })
    }

    fn adv(path: &str, value: Value, promo: Option<(&str, Option<i64>, Option<i64>)>) -> Advice {
        Advice {
            observation: "o".into(),
            context: "c".into(),
            proposal: "p".into(),
            inventory_action: None,
            promo: promo.map(|(k, s, d)| super::super::advice::PromoProposal {
                alias: "Producto 2".into(),
                kind: k.into(),
                second_off_pct: s,
                direct_off_pct: d,
            }),
            cited: vec![Cited {
                path: path.into(),
                value,
            }],
        }
    }

    fn resp(advice: Vec<Advice>) -> AdviceResponse {
        AdviceResponse {
            insufficient_data: false,
            advice,
        }
    }

    #[test]
    fn keeps_advice_with_matching_citation() {
        let r = validate(
            resp(vec![adv("/pace/vs_baseline_bps", json!(-1800), None)]),
            &input(),
            1_500,
        );
        assert_eq!(r.advice.len(), 1);
    }

    #[test]
    fn drops_advice_with_wrong_or_missing_citation() {
        let r = validate(
            resp(vec![
                adv("/pace/vs_baseline_bps", json!(-1700), None),
                adv("/pace/nope", json!(1), None),
            ]),
            &input(),
            1_500,
        );
        assert!(r.advice.is_empty());
    }

    #[test]
    fn drops_promo_below_cost_or_margin() {
        let r = validate(
            resp(vec![adv(
                "/pace/vs_baseline_bps",
                json!(-1800),
                Some(("direct_discount", None, Some(60))),
            )]),
            &input(),
            1_500,
        );
        assert!(r.advice.is_empty());
    }

    #[test]
    fn keeps_allowed_promo_and_caps_at_three() {
        let a = || {
            adv(
                "/pace/vs_baseline_bps",
                json!(-1800),
                Some(("direct_discount", None, Some(10))),
            )
        };
        let r = validate(resp(vec![a(), a(), a(), a()]), &input(), 1_500);
        assert_eq!(r.advice.len(), 3);
    }

    #[test]
    fn insufficient_data_returns_no_advice() {
        let r = validate(
            AdviceResponse {
                insufficient_data: true,
                advice: vec![adv("/pace/vs_baseline_bps", json!(-1800), None)],
            },
            &input(),
            1_500,
        );
        assert!(r.insufficient_data && r.advice.is_empty());
    }
}

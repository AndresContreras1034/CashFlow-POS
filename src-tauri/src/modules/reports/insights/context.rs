use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const DEFAULT_MIN_MARGIN_BPS: i64 = 1_500;

/// Alias -> id real. Vive solo en memoria; NO se serializa.
#[derive(Default)]
pub struct AliasMap {
    prefix_counts: HashMap<&'static str, usize>,
    pub to_id: HashMap<String, i64>,
}

impl AliasMap {
    pub fn alias(&mut self, prefix: &'static str, id: i64) -> String {
        let n = self.prefix_counts.entry(prefix).or_insert(0);
        *n += 1;
        let a = format!("{prefix} {n}");
        self.to_id.insert(a.clone(), id);
        a
    }
}

#[derive(Serialize)]
pub struct Sufficiency {
    pub history_days: i64,
    pub uncosted_pct_bps: i64,
}

#[derive(Serialize)]
pub struct CommercialContext {
    pub as_of: String,
    pub pace: Value,
    pub calendar: Value,
    pub inventory: Value,
    pub promo_candidates: Value,
    pub settings: Value,
    pub data_sufficiency: Sufficiency,
}

pub fn fingerprint(ctx: &CommercialContext) -> Result<String, serde_json::Error> {
    let v = serde_json::to_value(ctx)?;
    let bytes = serde_json::to_vec(&v)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(pace: i64) -> CommercialContext {
        CommercialContext {
            as_of: "2026-10-15".into(),
            pace: json!({"vs_baseline_bps": pace}),
            calendar: json!([]),
            inventory: json!({}),
            promo_candidates: json!([]),
            settings: json!({"min_margin_bps": 1500}),
            data_sufficiency: Sufficiency {
                history_days: 100,
                uncosted_pct_bps: 0,
            },
        }
    }

    #[test]
    fn same_input_same_hash() {
        assert_eq!(
            fingerprint(&ctx(-1800)).unwrap(),
            fingerprint(&ctx(-1800)).unwrap()
        );
    }

    #[test]
    fn different_input_different_hash() {
        assert_ne!(
            fingerprint(&ctx(-1800)).unwrap(),
            fingerprint(&ctx(500)).unwrap()
        );
    }

    #[test]
    fn aliases_are_sequential_and_mapped() {
        let mut m = AliasMap::default();
        assert_eq!(m.alias("Producto", 77), "Producto 1");
        assert_eq!(m.alias("Producto", 12), "Producto 2");
        assert_eq!(m.to_id["Producto 2"], 12);
    }

    #[test]
    fn json_has_no_real_names() {
        let s = serde_json::to_string(&ctx(0)).unwrap();
        for forbidden in ["Juan", "Perez", "@", "NIT", "phone"] {
            assert!(!s.contains(forbidden), "filtró: {forbidden}");
        }
    }
}

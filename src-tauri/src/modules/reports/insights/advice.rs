use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cited {
    pub path: String,
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromoProposal {
    pub alias: String,
    pub kind: String,
    pub second_off_pct: Option<i64>,
    pub direct_off_pct: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Advice {
    pub observation: String,
    pub context: String,
    pub proposal: String,
    pub inventory_action: Option<String>,
    pub promo: Option<PromoProposal>,
    pub cited: Vec<Cited>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdviceResponse {
    pub insufficient_data: bool,
    pub advice: Vec<Advice>,
}

use std::collections::HashMap;

use chrono::Duration;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    errors::app_error::AppError,
    modules::{
        commercial_calendar::service as calendar_service,
        reports::{
            dto::ReportFilter,
            insights::{
                combos::{self, Item},
                context::{AliasMap, CommercialContext, Sufficiency, DEFAULT_MIN_MARGIN_BPS},
                cover::{days_cover, shortage_risk, Cover},
                queries,
            },
            service as reports_service,
        },
    },
};

pub struct BuiltContext {
    pub context: CommercialContext,
    pub product_names: HashMap<String, String>,
}

fn variant_display_name(
    name: String,
    brand: Option<String>,
    attributes: Value,
    sku: Option<String>,
) -> String {
    let mut details = Vec::new();
    if let Some(brand) = brand.filter(|brand| !brand.trim().is_empty()) {
        details.push(brand);
    }
    if let Some(attributes) = attributes.as_object() {
        details.extend(attributes.iter().map(|(key, value)| {
            let value = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            format!("{key}: {value}")
        }));
    }
    if let Some(sku) = sku.filter(|sku| !sku.trim().is_empty()) {
        details.push(format!("SKU {sku}"));
    }
    if details.is_empty() {
        name
    } else {
        format!("{name} ({})", details.join(", "))
    }
}

fn alias_for(
    variant_id: i64,
    alias_map: &mut AliasMap,
    aliases: &mut HashMap<i64, String>,
) -> String {
    if let Some(alias) = aliases.get(&variant_id) {
        return alias.clone();
    }
    let alias = alias_map.alias("Producto", variant_id);
    aliases.insert(variant_id, alias.clone());
    alias
}

fn scenario_value(scenario: &super::promo::Scenario) -> Value {
    json!({
        "kind": scenario.kind,
        "profit": scenario.profit,
        "allowed": scenario.allowed,
        "breakeven_x100": scenario.breakeven_x100,
    })
}

pub async fn build_context(pool: &PgPool) -> Result<BuiltContext, AppError> {
    let pace = calendar_service::current_sales_pace(pool, 28).await?;
    let as_of = pace.as_of;
    let events = calendar_service::upcoming_events(pool, 90).await?;
    let from_90d = as_of - Duration::days(89);
    let from_28d = as_of - Duration::days(27);

    let stars = queries::fetch_star_variants(pool, from_90d, as_of)
        .await
        .map_err(AppError::from)?;
    let star_ids: Vec<i64> = stars.iter().map(|(variant_id, _)| *variant_id).collect();
    let units = queries::fetch_units_28d(pool, from_28d, as_of, &star_ids)
        .await
        .map_err(AppError::from)?;
    let stock = queries::fetch_stock(pool, &star_ids)
        .await
        .map_err(AppError::from)?;
    let history_days = queries::fetch_history_days(pool, as_of)
        .await
        .map_err(AppError::from)?;
    let uncosted_pct_bps = queries::fetch_uncosted_pct_bps(pool, from_90d, as_of)
        .await
        .map_err(AppError::from)?;

    let stale = reports_service::get_stale_products(
        pool,
        ReportFilter {
            date_from: from_90d,
            date_to: as_of,
            granularity: None,
            compare_previous: false,
            compare_year_over_year: false,
            category_id: None,
            limit: Some(100),
            rank_by: None,
        },
    )
    .await?;

    let mut variant_ids = star_ids.clone();
    variant_ids.extend(stale.iter().map(|row| i64::from(row.variant_id)));
    variant_ids.sort_unstable();
    variant_ids.dedup();
    let items_by_id: HashMap<i64, Item> = queries::fetch_combo_items(pool, &variant_ids)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|item| (item.variant_id, item))
        .collect();

    let star_items: Vec<Item> = star_ids
        .iter()
        .filter_map(|variant_id| items_by_id.get(variant_id).cloned())
        .collect();
    let stale_items: Vec<Item> = stale
        .iter()
        .filter_map(|row| items_by_id.get(&i64::from(row.variant_id)).cloned())
        .collect();

    let mut alias_map = AliasMap::default();
    let mut aliases = HashMap::new();
    for variant_id in &star_ids {
        alias_for(*variant_id, &mut alias_map, &mut aliases);
    }
    for row in &stale {
        alias_for(i64::from(row.variant_id), &mut alias_map, &mut aliases);
    }
    let display_names_by_id = queries::fetch_variant_display_names(pool, &variant_ids)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|(variant_id, name, brand, attributes, sku)| {
            (
                variant_id,
                variant_display_name(name, brand, attributes, sku),
            )
        })
        .collect::<HashMap<_, _>>();
    let product_names = alias_map
        .to_id
        .iter()
        .filter_map(|(alias, variant_id)| {
            display_names_by_id
                .get(variant_id)
                .map(|name| (alias.clone(), name.clone()))
        })
        .collect();

    let days_to_event = events.first().map(|event| event.in_days);
    let coverage: Vec<Value> = stars
        .iter()
        .map(|(variant_id, _)| {
            let cover = days_cover(
                stock.get(variant_id).copied().unwrap_or(0),
                units.get(variant_id).copied().unwrap_or(0),
                28,
                history_days,
            );
            let days = match cover {
                Cover::Days(days) => Some(days),
                Cover::NoSales | Cover::Insufficient => None,
            };
            let risk = days_to_event
                .map(|days_to_event| shortage_risk(cover, days_to_event))
                .unwrap_or(false);
            json!({
                "alias": aliases[variant_id],
                "days_cover": days,
                "risk": risk,
            })
        })
        .collect();

    let mut promo_candidates: Vec<Value> = Vec::new();
    for variant_id in &star_ids {
        let Some(item) = items_by_id.get(variant_id) else {
            continue;
        };
        if item.cost <= 0 {
            continue;
        }
        let scenarios: Vec<Value> =
            super::promo::single_product(item.price, item.cost, DEFAULT_MIN_MARGIN_BPS, 50, 10)
                .iter()
                .map(scenario_value)
                .collect();
        promo_candidates.push(json!({
            "alias": aliases[variant_id],
            "price": item.price,
            "cost": item.cost,
            "scenarios": scenarios,
        }));
    }

    let combo_result = combos::pick_combos(&stale_items, &star_items, 10, DEFAULT_MIN_MARGIN_BPS);
    for candidate in combo_result.candidates {
        let stale_alias = alias_for(candidate.stale_variant_id, &mut alias_map, &mut aliases);
        let star_alias = alias_for(candidate.star_variant_id, &mut alias_map, &mut aliases);
        let scenario = scenario_value(&candidate.scenario);
        promo_candidates.push(json!({
            "alias": format!("{stale_alias} + {star_alias}"),
            "scenarios": [scenario],
        }));
    }

    let calendar_status = if pace.year_ago_date.is_some() {
        "ok"
    } else {
        "insufficient"
    };
    let calendar: Vec<Value> = events
        .iter()
        .map(|event| {
            json!({
                "name": event.name,
                "kind": event.kind,
                "in_days": event.in_days,
                "last_year": { "status": calendar_status },
            })
        })
        .collect();
    let pace_value = match pace.vs_baseline_bps {
        Some(vs_baseline_bps) => json!({
            "status": "ok",
            "days": pace.days,
            "vs_baseline_bps": vs_baseline_bps,
        }),
        None => json!({ "status": "insufficient" }),
    };
    let (stale_value, stale_rows) = stale
        .first()
        .map(|row| (row.total_value_at_cost, row.total_rows))
        .unwrap_or((0, 0));

    Ok(BuiltContext {
        context: CommercialContext {
            as_of: as_of.to_string(),
            pace: pace_value,
            calendar: json!(calendar),
            inventory: json!({
                "coverage": coverage,
                "stale": {
                    "total_value_cents": stale_value,
                    "rows": stale_rows,
                },
            }),
            promo_candidates: json!(promo_candidates),
            settings: json!({ "min_margin_bps": DEFAULT_MIN_MARGIN_BPS }),
            data_sufficiency: Sufficiency {
                history_days,
                uncosted_pct_bps,
            },
        },
        product_names,
    })
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsufficientReason {
    RangeTooShort,
    NoCogs,
    LowCostCoverage,
    ImplausibleResult,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InventoryDays {
    Value(i64),
    Insufficient(InsufficientReason),
}

#[derive(Debug, Clone, Copy)]
pub struct DaysInput {
    pub inventory_value_cents: i64,
    pub cogs_cents: i64,
    pub range_days: i64,
    pub units_sold: i64,
    pub units_uncosted: i64,
}

const MIN_RANGE_DAYS: i64 = 28;
const MIN_COST_COVERAGE_PCT: i64 = 80;
const MAX_PLAUSIBLE_DAYS: i64 = 3650;

pub fn inventory_days(input: &DaysInput) -> InventoryDays {
    if input.range_days < MIN_RANGE_DAYS {
        return InventoryDays::Insufficient(InsufficientReason::RangeTooShort);
    }
    if input.cogs_cents <= 0 {
        return InventoryDays::Insufficient(InsufficientReason::NoCogs);
    }
    if input.units_sold > 0 {
        let units_costed = i128::from(input.units_sold) - i128::from(input.units_uncosted);
        if units_costed * 100 < i128::from(input.units_sold) * i128::from(MIN_COST_COVERAGE_PCT) {
            return InventoryDays::Insufficient(InsufficientReason::LowCostCoverage);
        }
    }

    let days = (i128::from(input.inventory_value_cents) * i128::from(input.range_days))
        / i128::from(input.cogs_cents);
    if days < 0 || days > i128::from(MAX_PLAUSIBLE_DAYS) {
        return InventoryDays::Insufficient(InsufficientReason::ImplausibleResult);
    }
    InventoryDays::Value(days as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> DaysInput {
        DaysInput {
            inventory_value_cents: 3_000_000,
            cogs_cents: 1_000_000,
            range_days: 30,
            units_sold: 100,
            units_uncosted: 0,
        }
    }

    #[test]
    fn calculates_plausible_inventory_days() {
        assert_eq!(inventory_days(&base()), InventoryDays::Value(90));
    }

    #[test]
    fn short_ranges_are_insufficient() {
        let mut input = base();
        input.range_days = 27;
        assert_eq!(
            inventory_days(&input),
            InventoryDays::Insufficient(InsufficientReason::RangeTooShort),
        );
    }

    #[test]
    fn zero_cogs_are_insufficient() {
        let mut input = base();
        input.cogs_cents = 0;
        assert_eq!(
            inventory_days(&input),
            InventoryDays::Insufficient(InsufficientReason::NoCogs),
        );
    }

    #[test]
    fn low_cost_coverage_is_insufficient() {
        let mut input = base();
        input.units_uncosted = 21;
        assert_eq!(
            inventory_days(&input),
            InventoryDays::Insufficient(InsufficientReason::LowCostCoverage),
        );
    }

    #[test]
    fn implausibly_large_results_are_hidden() {
        let mut input = base();
        input.cogs_cents = 2;
        assert_eq!(
            inventory_days(&input),
            InventoryDays::Insufficient(InsufficientReason::ImplausibleResult),
        );
    }

    #[test]
    fn exactly_eighty_percent_cost_coverage_is_accepted() {
        let mut input = base();
        input.units_sold = 100;
        input.units_uncosted = 20;
        assert_eq!(inventory_days(&input), InventoryDays::Value(90));
    }
}

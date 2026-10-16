pub const MAX_REGEN: i32 = 3;
pub const SCOPES: [&str; 4] = ["dashboard", "products", "inventory", "payables"];

pub fn valid_scope(scope: &str) -> bool {
    SCOPES.contains(&scope)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    Disabled,
    TokenCap,
    RegenLimit,
    SameInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Generate { regen_index: i32 },
    Blocked(BlockReason),
}

#[derive(Debug, Clone, Default)]
pub struct DayState {
    /// Tokens usados hoy en todos los scopes (tope diario global).
    pub tokens_today: i64,
    /// Mayor regen_index de hoy para el scope, de cualquier estado.
    pub last_index: Option<i32>,
    /// Huella de la última fila ok de hoy para el scope.
    pub last_ok_hash: Option<String>,
}

pub fn decide(enabled: bool, cap: i64, state: &DayState, current_hash: &str) -> Decision {
    if !enabled {
        return Decision::Blocked(BlockReason::Disabled);
    }
    if state.tokens_today >= cap {
        return Decision::Blocked(BlockReason::TokenCap);
    }
    if state.last_ok_hash.as_deref() == Some(current_hash) {
        return Decision::Blocked(BlockReason::SameInput);
    }
    match state.last_index {
        None => Decision::Generate { regen_index: 0 },
        Some(i) if i >= MAX_REGEN => Decision::Blocked(BlockReason::RegenLimit),
        Some(i) => Decision::Generate { regen_index: i + 1 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(tokens: i64, idx: Option<i32>, hash: Option<&str>) -> DayState {
        DayState {
            tokens_today: tokens,
            last_index: idx,
            last_ok_hash: hash.map(String::from),
        }
    }

    #[test]
    fn disabled_blocks() {
        assert_eq!(
            decide(false, 1000, &st(0, None, None), "h"),
            Decision::Blocked(BlockReason::Disabled)
        );
    }

    #[test]
    fn token_cap_blocks() {
        assert_eq!(
            decide(true, 1000, &st(1000, None, None), "h"),
            Decision::Blocked(BlockReason::TokenCap)
        );
    }

    #[test]
    fn first_generation_is_index_zero() {
        assert_eq!(
            decide(true, 1000, &st(0, None, None), "h"),
            Decision::Generate { regen_index: 0 }
        );
    }

    #[test]
    fn same_hash_is_blocked() {
        assert_eq!(
            decide(true, 1000, &st(10, Some(0), Some("h")), "h"),
            Decision::Blocked(BlockReason::SameInput)
        );
    }

    #[test]
    fn changed_hash_regenerates_with_next_index() {
        assert_eq!(
            decide(true, 1000, &st(10, Some(0), Some("old")), "new"),
            Decision::Generate { regen_index: 1 }
        );
    }

    #[test]
    fn regen_limit_is_three() {
        assert_eq!(
            decide(true, 1000, &st(10, Some(2), Some("old")), "new"),
            Decision::Generate { regen_index: 3 }
        );
        assert_eq!(
            decide(true, 1000, &st(10, Some(3), Some("old")), "new"),
            Decision::Blocked(BlockReason::RegenLimit)
        );
    }

    #[test]
    fn failed_first_attempt_counts_as_regen() {
        assert_eq!(
            decide(true, 1000, &st(10, Some(0), None), "h"),
            Decision::Generate { regen_index: 1 }
        );
    }
}

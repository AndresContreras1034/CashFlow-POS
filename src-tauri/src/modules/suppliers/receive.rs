//! Receiving a purchase list atomically adds stock, updates variant costs, and closes the list.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::audit::{actor::declared_actor, repository as audit_repo};
use crate::modules::debts::repository::currency_decimals;
use crate::modules::inventory::{
    dto::StockEntryDto, models::MovementType, repository as inventory_repo,
};
use crate::modules::suppliers::purchase_lists::{
    list_event, lock_list, PurchaseList, LIST_COLS, MAX_QTY,
};
use crate::types::money::{divide_to_currency, round_to_currency};

const MAX_UNIT_COST: i64 = 100_000_000_000_000;
const AUDIT_CHANGES_LIMIT: usize = 100;

#[derive(Debug, Clone, Deserialize)]
pub struct ReceiveLineDto {
    pub item_id: i32,
    /// 0 means the item did not arrive.
    pub quantity: i32,
    /// Actual unit cost; `None` means unknown and leaves the current cost unchanged.
    pub unit_cost: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceiveListDto {
    pub lines: Vec<ReceiveLineDto>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostChange {
    pub variant_id: i32,
    pub product_name: String,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiveResult {
    pub list: PurchaseList,
    pub received_lines: usize,
    pub skipped_lines: usize,
    pub units: i64,
    pub total_cost: i64,
    pub cost_changes: Vec<CostChange>,
}

fn overflow() -> AppError {
    AppError::validation("El costo de la compra supera el máximo permitido")
}

/// Unit cost is optional; when present it must be positive, bounded, and currency-precise.
pub fn validate_unit_cost(cost: Option<i64>, decimals: u8) -> Result<Option<i64>, AppError> {
    let Some(value) = cost else { return Ok(None) };
    if value <= 0 {
        return Err(AppError::validation(
            "El costo unitario debe ser mayor a cero o quedar vacío",
        ));
    }
    if value > MAX_UNIT_COST {
        return Err(AppError::validation(
            "El costo unitario supera el máximo permitido",
        ));
    }
    if round_to_currency(value, decimals) != value {
        return Err(AppError::validation(&if decimals == 0 {
            "El costo debe ser un valor entero, sin decimales".to_string()
        } else {
            format!("El costo no puede tener más de {decimals} decimales")
        }));
    }
    Ok(Some(value))
}

/// Requires every list line exactly once, valid quantities and at least one received item.
pub fn validate_lines(
    lines: &[ReceiveLineDto],
    item_ids: &[i32],
    decimals: u8,
) -> Result<(), AppError> {
    let expected: HashSet<i32> = item_ids.iter().copied().collect();
    let mut seen = HashSet::new();
    for line in lines {
        if !expected.contains(&line.item_id) {
            return Err(AppError::validation(
                "Hay una línea que no pertenece a esta lista. Recarga e inténtalo de nuevo",
            ));
        }
        if !seen.insert(line.item_id) {
            return Err(AppError::validation("Una línea aparece repetida"));
        }
        if !(0..=MAX_QTY).contains(&line.quantity) {
            return Err(AppError::validation(&format!(
                "La cantidad recibida debe estar entre 0 y {MAX_QTY}"
            )));
        }
        validate_unit_cost(line.unit_cost, decimals)?;
    }
    if seen.len() != expected.len() {
        return Err(AppError::validation(
            "Faltan líneas de la lista. Recarga e inténtalo de nuevo",
        ));
    }
    if lines.iter().all(|line| line.quantity == 0) {
        return Err(AppError::validation(
            "Indica la cantidad recibida de al menos un producto",
        ));
    }
    Ok(())
}

/// Weighted average of current on-hand stock and the received purchase, rounded to currency.
pub fn weighted_average_cost(
    stock_before: i32,
    old_cost: i64,
    quantity: i32,
    unit_cost: i64,
    decimals: u8,
) -> Result<i64, AppError> {
    let on_hand = i128::from(stock_before.max(0));
    if on_hand == 0 || old_cost <= 0 {
        return Ok(unit_cost);
    }
    let numerator = on_hand * i128::from(old_cost) + i128::from(quantity) * i128::from(unit_cost);
    let denominator = on_hand + i128::from(quantity);
    let numerator = i64::try_from(numerator).map_err(|_| overflow())?;
    let denominator = i64::try_from(denominator).map_err(|_| overflow())?;
    divide_to_currency(numerator, denominator, decimals).ok_or_else(overflow)
}

pub async fn receive_purchase_list(
    pool: &PgPool,
    list_id: i32,
    dto: ReceiveListDto,
) -> Result<ReceiveResult, AppError> {
    let decimals = currency_decimals(pool).await.map_err(AppError::from)?;
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let list = lock_list(&mut tx, list_id).await?;
    match list.status.as_str() {
        "draft" | "ordered" => {}
        "received" => return Err(AppError::validation("Esta lista ya fue recibida")),
        _ => {
            return Err(AppError::validation(
                "Una lista cancelada no se puede recibir",
            ))
        }
    }

    let items: Vec<(i32, i32, Option<i32>, String)> = sqlx::query_as(
        "SELECT i.id, i.variant_id, i.supplier_id, p.name
         FROM purchase_list_items i
         JOIN product_variants v ON v.id = i.variant_id
         JOIN products p ON p.id = v.product_id
         WHERE i.list_id = $1
         ORDER BY i.variant_id",
    )
    .bind(list_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::from)?;
    if items.is_empty() {
        return Err(AppError::validation("La lista está vacía"));
    }

    let item_ids: Vec<i32> = items.iter().map(|row| row.0).collect();
    validate_lines(&dto.lines, &item_ids, decimals)?;
    let by_item: HashMap<i32, &ReceiveLineDto> =
        dto.lines.iter().map(|line| (line.item_id, line)).collect();

    let note = format!("Compra: lista #{} «{}»", list.id, list.title);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let (mut received, mut skipped, mut units, mut total_cost) = (0usize, 0usize, 0i64, 0i64);
    let mut cost_changes: Vec<CostChange> = Vec::new();

    for (item_id, variant_id, supplier_id, product_name) in &items {
        let line = by_item[item_id];
        if line.quantity == 0 {
            sqlx::query(
                "UPDATE purchase_list_items
                 SET received_quantity = 0, received_unit_cost = NULL WHERE id = $1",
            )
            .bind(item_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::from)?;
            skipped += 1;
            continue;
        }

        let variant = inventory_repo::apply_stock_entry(
            &mut tx,
            StockEntryDto {
                variant_id: *variant_id,
                quantity: line.quantity,
                unit_cost: line.unit_cost,
                notes: Some(note.clone()),
                created_by: actor.clone(),
            },
            MovementType::Purchase,
        )
        .await
        .map_err(AppError::from)?;

        if let Some(unit_cost) = line.unit_cost {
            let stock_before = variant.stock - line.quantity;
            let new_cost = weighted_average_cost(
                stock_before,
                variant.cost,
                line.quantity,
                unit_cost,
                decimals,
            )?;
            if new_cost != variant.cost {
                sqlx::query("UPDATE product_variants SET cost = $2 WHERE id = $1")
                    .bind(variant_id)
                    .bind(new_cost)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::from)?;
                cost_changes.push(CostChange {
                    variant_id: *variant_id,
                    product_name: product_name.clone(),
                    from: variant.cost,
                    to: new_cost,
                });
            }
            total_cost = i64::from(line.quantity)
                .checked_mul(unit_cost)
                .and_then(|line_total| total_cost.checked_add(line_total))
                .ok_or_else(overflow)?;
        }

        sqlx::query(
            "UPDATE purchase_list_items
             SET received_quantity = $2, received_unit_cost = $3 WHERE id = $1",
        )
        .bind(item_id)
        .bind(line.quantity)
        .bind(line.unit_cost)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

        if let Some(supplier_id) = supplier_id {
            sqlx::query(
                "INSERT INTO supplier_variants
                     (supplier_id, variant_id, last_cost, last_purchase_at, created_by)
                 VALUES ($1, $2, $3, NOW(), $4)
                 ON CONFLICT (supplier_id, variant_id) DO UPDATE
                 SET last_cost = COALESCE(EXCLUDED.last_cost, supplier_variants.last_cost),
                     last_purchase_at = NOW()",
            )
            .bind(supplier_id)
            .bind(variant_id)
            .bind(line.unit_cost)
            .bind(&created_by)
            .execute(&mut *tx)
            .await
            .map_err(AppError::from)?;
        }

        received += 1;
        units += i64::from(line.quantity);
    }

    let after = sqlx::query_as::<_, PurchaseList>(&format!(
        "UPDATE purchase_lists SET status = 'received', received_at = NOW()
         WHERE id = $1 RETURNING {LIST_COLS}"
    ))
    .bind(list_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let shown: Vec<&CostChange> = cost_changes.iter().take(AUDIT_CHANGES_LIMIT).collect();
    audit_repo::insert_event(
        &mut tx,
        list_event(
            actor,
            "purchase_list_receive",
            after.id,
            format!(
                "Lista de compras #{} recibida: {} productos, {} unidades",
                after.id, received, units
            ),
            Some(json!({ "status": { "from": list.status, "to": after.status } })),
            json!({
                "title": after.title,
                "lines_received": received,
                "lines_skipped": skipped,
                "units": units,
                "total_cost": total_cost,
                "cost_changes": shown,
                "cost_changes_truncated": cost_changes.len() > AUDIT_CHANGES_LIMIT,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(ReceiveResult {
        list: after,
        received_lines: received,
        skipped_lines: skipped,
        units,
        total_cost,
        cost_changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(item_id: i32, quantity: i32, unit_cost: Option<i64>) -> ReceiveLineDto {
        ReceiveLineDto {
            item_id,
            quantity,
            unit_cost,
        }
    }

    #[test]
    fn weighted_average_mixes_existing_stock_with_the_purchase() {
        assert_eq!(
            weighted_average_cost(2, 100_000, 8, 150_000, 0).unwrap(),
            140_000
        );
        assert_eq!(
            weighted_average_cost(10, 200_000, 10, 100_000, 0).unwrap(),
            150_000
        );
    }

    #[test]
    fn without_usable_stock_or_previous_cost_the_purchase_cost_wins() {
        assert_eq!(
            weighted_average_cost(0, 100_000, 5, 150_000, 0).unwrap(),
            150_000
        );
        assert_eq!(
            weighted_average_cost(-4, 100_000, 5, 150_000, 0).unwrap(),
            150_000
        );
        assert_eq!(weighted_average_cost(7, 0, 5, 150_000, 0).unwrap(), 150_000);
    }

    #[test]
    fn the_average_is_rounded_to_the_currency_precision() {
        assert_eq!(
            weighted_average_cost(1, 10_000, 1, 10_100, 0).unwrap(),
            10_100
        );
        assert_eq!(
            weighted_average_cost(1, 10_000, 1, 10_100, 2).unwrap(),
            10_050
        );
    }

    #[test]
    fn absurd_numbers_report_an_error_instead_of_panicking() {
        assert!(weighted_average_cost(i32::MAX, MAX_UNIT_COST, 1, MAX_UNIT_COST, 0).is_err());
    }

    #[test]
    fn unit_cost_is_empty_or_positive_bounded_and_within_precision() {
        assert_eq!(validate_unit_cost(None, 0).unwrap(), None);
        assert_eq!(validate_unit_cost(Some(150_000), 0).unwrap(), Some(150_000));
        assert!(validate_unit_cost(Some(0), 0).is_err());
        assert!(validate_unit_cost(Some(-5), 0).is_err());
        assert!(validate_unit_cost(Some(150_050), 0).is_err());
        assert!(validate_unit_cost(Some(150_050), 2).is_ok());
        assert!(validate_unit_cost(Some(MAX_UNIT_COST + 100), 0).is_err());
    }

    #[test]
    fn every_line_must_appear_once_and_at_least_one_must_arrive() {
        let ids = [1, 2];
        assert!(validate_lines(&[line(1, 3, None), line(2, 0, None)], &ids, 0).is_ok());
        assert!(validate_lines(&[line(1, 3, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, 3, None), line(1, 3, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, 1, None), line(9, 1, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, 0, None), line(2, 0, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, -1, None), line(2, 1, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, MAX_QTY + 1, None), line(2, 1, None)], &ids, 0).is_err());
        assert!(validate_lines(&[line(1, 1, Some(0)), line(2, 1, None)], &ids, 0).is_err());
    }
}

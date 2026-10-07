use sqlx::PgPool;

use crate::modules::cash::{
    dto::CreateMovementDto, models::CashMovementType, repository as cash_repo,
};
use crate::modules::inventory::{
    dto::StockOutDto, models::MovementType, repository as inventory_repo,
};
use crate::modules::sales::{
    dto::{CreateSaleDto, SaleFilterDto},
    models::{PaymentMethod, Sale, SaleDetail, SaleItem, SalePayment, SaleStatus},
};
use crate::types::money::tax_included_in;

/// Errores de negocio propios de la confirmación de venta —
/// se distinguen de sqlx::Error para poder dar mensajes claros en el service.
pub enum CreateSaleError {
    Db(sqlx::Error),
    NoOpenCashSession,
    PaymentMismatch,
    InvalidDiscount,
    AmountOverflow,
    InsufficientStock {
        variant_id: i32,
        available: i32,
        requested: i32,
    },
}

fn allocate_proportionally(amount: i64, weights: &[i64]) -> Vec<i64> {
    let total_weight: i128 = weights.iter().map(|weight| (*weight).max(0) as i128).sum();
    if amount <= 0 || total_weight == 0 {
        return vec![0; weights.len()];
    }

    let mut shares = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    let mut allocated = 0_i64;

    for (index, weight) in weights.iter().enumerate() {
        let numerator = amount as i128 * (*weight).max(0) as i128;
        let share = (numerator / total_weight) as i64;
        allocated += share;
        shares.push(share);
        remainders.push((index, numerator % total_weight));
    }

    remainders.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (index, _) in remainders.into_iter().take((amount - allocated) as usize) {
        shares[index] += 1;
    }

    shares
}

impl From<sqlx::Error> for CreateSaleError {
    fn from(e: sqlx::Error) -> Self {
        CreateSaleError::Db(e)
    }
}

/// Crea la venta completa — venta, ítems, pagos, salida de inventario y
/// movimiento de caja si aplica — dentro de UNA sola transacción. Si algo
/// falla en cualquier paso, todo se revierte automáticamente al dropear `tx`.
pub async fn create_sale(pool: &PgPool, dto: CreateSaleDto) -> Result<SaleDetail, CreateSaleError> {
    let mut tx = pool.begin().await?;

    let created_by = dto
        .created_by
        .clone()
        .unwrap_or_else(|| "system".to_string());
    let global_discount = dto.discount.unwrap_or(0);
    if global_discount < 0 {
        return Err(CreateSaleError::InvalidDiscount);
    }

    let settings = sqlx::query!("SELECT currency, tax_rate_bps FROM app_settings WHERE id = 1")
        .fetch_one(&mut *tx)
        .await?;
    let currency_decimals = if settings.currency == "COP" { 0 } else { 2 };

    // 1. Precio server-side por cada ítem — nunca se confía en el precio del cliente
    struct LineItem {
        variant_id: i32,
        quantity: i32,
        unit_price: i64,
        discount: i64,
        subtotal: i64,
        tax: i64,
    }
    let mut lines = Vec::new();
    let mut subtotal_total: i64 = 0;

    for item in &dto.items {
        let price = sqlx::query_scalar!(
            "SELECT price FROM product_variants WHERE id = $1",
            item.variant_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let discount = item.discount.unwrap_or(0);
        let gross = price
            .checked_mul(item.quantity as i64)
            .ok_or(CreateSaleError::AmountOverflow)?;
        if discount < 0 || discount > gross {
            return Err(CreateSaleError::InvalidDiscount);
        }
        let line_subtotal = gross - discount;

        subtotal_total = subtotal_total
            .checked_add(line_subtotal)
            .ok_or(CreateSaleError::AmountOverflow)?;
        lines.push(LineItem {
            variant_id: item.variant_id,
            quantity: item.quantity,
            unit_price: price,
            discount,
            subtotal: line_subtotal,
            tax: 0,
        });
    }

    if global_discount > subtotal_total {
        return Err(CreateSaleError::InvalidDiscount);
    }
    let total = subtotal_total - global_discount;
    let total_tax = tax_included_in(total, settings.tax_rate_bps, currency_decimals);
    let allocated_discounts = allocate_proportionally(
        global_discount,
        &lines.iter().map(|line| line.subtotal).collect::<Vec<_>>(),
    );
    let discounted_line_totals: Vec<i64> = lines
        .iter()
        .zip(allocated_discounts)
        .map(|(line, discount)| line.subtotal - discount)
        .collect();
    let currency_step = 10_i64.pow(2_u32 - currency_decimals as u32);
    let allocated_taxes =
        allocate_proportionally(total_tax / currency_step, &discounted_line_totals)
            .into_iter()
            .map(|tax_units| tax_units * currency_step)
            .collect::<Vec<_>>();
    for (line, tax) in lines.iter_mut().zip(allocated_taxes) {
        line.tax = tax;
    }
    let payments_total = dto.payments.iter().try_fold(0_i64, |sum, payment| {
        sum.checked_add(payment.amount)
            .ok_or(CreateSaleError::AmountOverflow)
    })?;

    if payments_total != total {
        return Err(CreateSaleError::PaymentMismatch);
    }

    // 2. Insertar sale
    let sale = sqlx::query_as!(
        Sale,
        r#"INSERT INTO sales (customer_id, subtotal, tax, discount, total, status, notes, created_by)
           VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7)
           RETURNING id, customer_id, subtotal, tax, discount, total,
                     status AS "status: SaleStatus", notes, created_by, created_at"#,
        dto.customer_id,
        subtotal_total,
        total_tax,
        global_discount,
        total,
        dto.notes,
        created_by
    )
    .fetch_one(&mut *tx)
    .await?;

    // 3. Insertar sale_items y descontar stock de cada uno
    let mut items = Vec::new();
    for line in lines {
        let saved_item = sqlx::query_as!(
            SaleItem,
            "INSERT INTO sale_items (sale_id, variant_id, quantity, unit_price, discount, tax, subtotal)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
            sale.id,
            line.variant_id,
            line.quantity,
            line.unit_price,
            line.discount,
            line.tax,
            line.subtotal
        )
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query!(
            "SELECT stock, allow_negative FROM product_variants WHERE id = $1 FOR UPDATE",
            line.variant_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if row.stock < line.quantity && !row.allow_negative {
            return Err(CreateSaleError::InsufficientStock {
                variant_id: line.variant_id,
                available: row.stock,
                requested: line.quantity,
            });
        }

        inventory_repo::apply_stock_out(
            &mut tx,
            StockOutDto {
                variant_id: line.variant_id,
                quantity: line.quantity,
                reason: None,
                notes: Some(format!("Venta #{}", sale.id)),
                created_by: Some(created_by.clone()),
                sale_id: Some(sale.id),
            },
            MovementType::Sale,
        )
        .await?;

        items.push(saved_item);
    }

    // 4. Insertar sale_payments
    let mut payments = Vec::new();
    for p in &dto.payments {
        let saved = sqlx::query_as!(
            SalePayment,
            r#"INSERT INTO sale_payments (sale_id, method, amount)
               VALUES ($1, $2, $3)
               RETURNING id, sale_id, method AS "method: PaymentMethod", amount"#,
            sale.id,
            p.method as PaymentMethod,
            p.amount
        )
        .fetch_one(&mut *tx)
        .await?;
        payments.push(saved);
    }

    // 5. Pago en efectivo -> movimiento de caja en el turno abierto
    let cash_amount: i64 = dto
        .payments
        .iter()
        .filter(|p| p.method == PaymentMethod::Cash)
        .map(|p| p.amount)
        .sum();

    if cash_amount > 0 {
        let session_id = sqlx::query_scalar!("SELECT id FROM cash_sessions WHERE status = 'open'")
            .fetch_optional(&mut *tx)
            .await?;

        let Some(session_id) = session_id else {
            return Err(CreateSaleError::NoOpenCashSession);
        };

        cash_repo::create_movement(
            &mut tx,
            session_id,
            CreateMovementDto {
                movement_type: CashMovementType::SaleIn,
                amount: cash_amount,
                notes: Some(format!("Venta #{}", sale.id)),
                created_by: Some(created_by.clone()),
                sale_id: Some(sale.id),
            },
        )
        .await?;
    }

    tx.commit().await?;
    Ok(SaleDetail {
        sale,
        items,
        payments,
    })
}

pub async fn get_sale_by_id(pool: &PgPool, id: i32) -> Result<Option<SaleDetail>, sqlx::Error> {
    let sale = sqlx::query_as!(
        Sale,
        r#"SELECT id, customer_id, subtotal, tax, discount, total,
                  status AS "status: SaleStatus", notes, created_by, created_at
           FROM sales WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    let Some(sale) = sale else { return Ok(None) };

    let items = sqlx::query_as!(
        SaleItem,
        "SELECT * FROM sale_items WHERE sale_id = $1 ORDER BY id ASC",
        id
    )
    .fetch_all(pool)
    .await?;

    let payments = sqlx::query_as!(
        SalePayment,
        r#"SELECT id, sale_id, method AS "method: PaymentMethod", amount
           FROM sale_payments WHERE sale_id = $1"#,
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(Some(SaleDetail {
        sale,
        items,
        payments,
    }))
}

pub async fn list_sales(
    pool: &PgPool,
    filter: &SaleFilterDto,
) -> Result<(Vec<Sale>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as!(
        Sale,
        r#"SELECT id, customer_id, subtotal, tax, discount, total,
                  status AS "status: SaleStatus", notes, created_by, created_at
           FROM sales
           WHERE ($1::TEXT IS NULL OR status::TEXT = $1)
             AND ($2::INT  IS NULL OR customer_id = $2)
             AND ($3::DATE IS NULL OR created_at::DATE >= $3)
             AND ($4::DATE IS NULL OR created_at::DATE <= $4)
           ORDER BY created_at DESC
           LIMIT $5 OFFSET $6"#,
        filter.status,
        filter.customer_id,
        filter.date_from,
        filter.date_to,
        page_size,
        offset
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM sales
           WHERE ($1::TEXT IS NULL OR status::TEXT = $1)
             AND ($2::INT  IS NULL OR customer_id = $2)
             AND ($3::DATE IS NULL OR created_at::DATE >= $3)
             AND ($4::DATE IS NULL OR created_at::DATE <= $4)"#,
        filter.status,
        filter.customer_id,
        filter.date_from,
        filter.date_to
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proportional_allocation_preserves_amount_and_prioritizes_largest_remainders() {
        assert_eq!(allocate_proportionally(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(allocate_proportionally(19, &[100, 200]), vec![6, 13]);
    }

    #[test]
    fn discounts_and_tax_allocations_reconcile_with_sale_total() {
        let gross_lines = [10_001, 20_002];
        let sale_discount = 3_000;
        let discounts = allocate_proportionally(sale_discount, &gross_lines);
        let after_discount: Vec<i64> = gross_lines
            .iter()
            .zip(discounts)
            .map(|(gross, discount)| gross - discount)
            .collect();
        let sale_total: i64 = after_discount.iter().sum();
        let tax_total = tax_included_in(sale_total, 1900, 0);
        let line_taxes = allocate_proportionally(tax_total / 100, &after_discount)
            .into_iter()
            .map(|tax_units| tax_units * 100)
            .collect::<Vec<_>>();

        assert_eq!(line_taxes.iter().sum::<i64>(), tax_total);
        assert!(line_taxes.iter().all(|tax| tax % 100 == 0));
    }
}

use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};

use crate::modules::cash::{
    dto::CreateMovementDto, models::CashMovementType, repository as cash_repo,
};
use crate::modules::inventory::{
    dto::StockOutDto, models::MovementType, repository as inventory_repo,
};
use crate::modules::sales::{
    dto::{AssignSaleCustomerDto, CreateSaleDto, SaleFilterDto, VoidSaleDto},
    models::{PaymentMethod, Sale, SaleDetail, SaleItem, SalePayment, SaleStatus},
};
use crate::types::money::tax_included_in;

/// Errores de negocio propios de la confirmación de venta —
/// se distinguen de sqlx::Error para poder dar mensajes claros en el service.
pub enum CreateSaleError {
    Db(sqlx::Error),
    NoOpenCashSession,
    CustomerNotFound,
    CustomerInactive,
    CourtesyReasonRequired,
    CourtesyReasonTooLong,
    CourtesyZeroGross,
    CourtesyHasPayments,
    NoPayments,
    PaymentMismatch,
    CashReceivedWithoutCash,
    CashReceivedTooLow,
    CashReceivedPrecision,
    InvalidDiscount,
    AmountOverflow,
    VariantNotFound,
    VariantUnavailable,
    InsufficientStock {
        variant_id: i32,
        available: i32,
        requested: i32,
    },
}

pub enum VoidSaleError {
    Db(sqlx::Error),
    NotFound,
    NotCompleted(SaleStatus),
    CashSessionRequired,
}

pub enum AssignCustomerError {
    Db(sqlx::Error),
    SaleNotFound,
    CustomerNotFound,
    CustomerInactive,
}

impl From<sqlx::Error> for AssignCustomerError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error)
    }
}

impl From<sqlx::Error> for VoidSaleError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error)
    }
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

/// Crea la venta completa y la asocia al turno abierto dentro de una sola
/// transacción. Si cualquier paso falla, todo se revierte al dropear `tx`.
pub async fn create_sale(pool: &PgPool, dto: CreateSaleDto) -> Result<SaleDetail, CreateSaleError> {
    let mut tx = pool.begin().await?;

    if let Some(key) = dto.idempotency_key {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(key.to_string())
            .execute(&mut *tx)
            .await?;

        let existing: Option<i32> =
            sqlx::query_scalar("SELECT id FROM sales WHERE idempotency_key = $1")
                .bind(key)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some(sale_id) = existing {
            tx.rollback().await?;
            return get_sale_by_id(pool, sale_id)
                .await?
                .ok_or(CreateSaleError::Db(sqlx::Error::RowNotFound));
        }
    }

    let cash_session_id: i32 =
        sqlx::query_scalar("SELECT id FROM cash_sessions WHERE status = 'open' FOR UPDATE")
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(CreateSaleError::NoOpenCashSession)?;

    if let Some(customer_id) = dto.customer_id {
        let active: Option<bool> =
            sqlx::query_scalar("SELECT is_active FROM customers WHERE id = $1 FOR SHARE")
                .bind(customer_id)
                .fetch_optional(&mut *tx)
                .await?;
        match active {
            None => return Err(CreateSaleError::CustomerNotFound),
            Some(false) => return Err(CreateSaleError::CustomerInactive),
            Some(true) => {}
        }
    }

    let created_by = dto
        .created_by
        .clone()
        .unwrap_or_else(|| "system".to_string());
    let actor = declared_actor(&dto.created_by);
    let requested_discount = dto.discount.unwrap_or(0);
    if requested_discount < 0 {
        return Err(CreateSaleError::InvalidDiscount);
    }
    let requested_courtesy_reason = dto
        .courtesy_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(str::to_string);

    let settings =
        sqlx::query!("SELECT tax_rate_bps, currency_decimals FROM app_settings WHERE id = 1")
            .fetch_one(&mut *tx)
            .await?;
    let currency_decimals = settings.currency_decimals as u8;

    let mut variant_ids: Vec<i32> = dto.items.iter().map(|item| item.variant_id).collect();
    variant_ids.sort_unstable();
    variant_ids.dedup();

    let variants = sqlx::query!(
        r#"SELECT pv.id, pv.price,
                  pv.is_active AS "variant_active!",
                  p.is_active AS "product_active!"
           FROM product_variants pv
           JOIN products p ON p.id = pv.product_id
           WHERE pv.id = ANY($1)
           ORDER BY pv.id
           FOR UPDATE OF pv"#,
        &variant_ids
    )
    .fetch_all(&mut *tx)
    .await?;

    let variant_costs: HashMap<i32, i64> =
        sqlx::query_as::<_, (i32, i64)>("SELECT id, cost FROM product_variants WHERE id = ANY($1)")
            .bind(&variant_ids)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    if variants.len() != variant_ids.len() {
        return Err(CreateSaleError::VariantNotFound);
    }
    if variants
        .iter()
        .any(|variant| !variant.variant_active || !variant.product_active)
    {
        return Err(CreateSaleError::VariantUnavailable);
    }

    // 1. Precio server-side por cada ítem — nunca se confía en el precio del cliente
    struct LineItem {
        variant_id: i32,
        quantity: i32,
        unit_price: i64,
        unit_cost: Option<i64>,
        discount: i64,
        subtotal: i64,
        tax: i64,
    }
    let mut lines = Vec::new();
    let mut subtotal_total: i64 = 0;
    let mut gross_total: i64 = 0;
    let mut discount_lines: i64 = 0;

    for item in &dto.items {
        let price = variants
            .iter()
            .find(|variant| variant.id == item.variant_id)
            .ok_or(CreateSaleError::VariantNotFound)?
            .price;

        let discount = item.discount.unwrap_or(0);
        let gross = price
            .checked_mul(item.quantity as i64)
            .ok_or(CreateSaleError::AmountOverflow)?;
        gross_total = gross_total
            .checked_add(gross)
            .ok_or(CreateSaleError::AmountOverflow)?;
        if discount < 0 || discount > gross {
            return Err(CreateSaleError::InvalidDiscount);
        }
        discount_lines = discount_lines
            .checked_add(discount)
            .ok_or(CreateSaleError::AmountOverflow)?;
        let line_subtotal = gross - discount;

        subtotal_total = subtotal_total
            .checked_add(line_subtotal)
            .ok_or(CreateSaleError::AmountOverflow)?;
        lines.push(LineItem {
            variant_id: item.variant_id,
            quantity: item.quantity,
            unit_price: price,
            unit_cost: variant_costs
                .get(&item.variant_id)
                .copied()
                .filter(|cost| *cost > 0),
            discount,
            subtotal: line_subtotal,
            tax: 0,
        });
    }

    let global_discount = requested_discount;
    if global_discount > subtotal_total {
        return Err(CreateSaleError::InvalidDiscount);
    }
    let discount_total = discount_lines
        .checked_add(global_discount)
        .ok_or(CreateSaleError::AmountOverflow)?;
    let total = subtotal_total - global_discount;
    let courtesy_reason = if total == 0 {
        if gross_total == 0 {
            return Err(CreateSaleError::CourtesyZeroGross);
        }
        if !dto.payments.is_empty() {
            return Err(CreateSaleError::CourtesyHasPayments);
        }
        let reason = requested_courtesy_reason.ok_or(CreateSaleError::CourtesyReasonRequired)?;
        if reason.chars().count() > 200 {
            return Err(CreateSaleError::CourtesyReasonTooLong);
        }
        Some(reason)
    } else {
        if dto.payments.is_empty() {
            return Err(CreateSaleError::NoPayments);
        }
        None
    };
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

    let cash_net = dto
        .payments
        .iter()
        .filter(|payment| payment.method == PaymentMethod::Cash)
        .try_fold(0_i64, |sum, payment| {
            sum.checked_add(payment.amount)
                .ok_or(CreateSaleError::AmountOverflow)
        })?;
    let (cash_received, change_given) = if let Some(received) = dto.cash_received {
        if total == 0 || cash_net == 0 {
            return Err(CreateSaleError::CashReceivedWithoutCash);
        }
        if received < cash_net {
            return Err(CreateSaleError::CashReceivedTooLow);
        }
        let money_step = 10_i64.pow(2_u32 - currency_decimals as u32);
        if received % money_step != 0 {
            return Err(CreateSaleError::CashReceivedPrecision);
        }
        (
            Some(received),
            Some(
                received
                    .checked_sub(cash_net)
                    .ok_or(CreateSaleError::AmountOverflow)?,
            ),
        )
    } else {
        (None, None)
    };

    // 2. Insertar sale
    let sale = sqlx::query_as::<_, Sale>(
        r#"INSERT INTO sales
               (customer_id, subtotal, tax, discount, total, status, notes, courtesy_reason,
                cash_received, change_given, created_by, gross_semantics)
           VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7, $8, $9, $10, TRUE)
           RETURNING id, customer_id, subtotal, tax, discount, total, cash_received,
                     change_given, status, notes, courtesy_reason, created_by, created_at,
                     voided_at, void_reason, voided_by"#,
    )
    .bind(dto.customer_id)
    .bind(gross_total)
    .bind(total_tax)
    .bind(discount_total)
    .bind(total)
    .bind(dto.notes)
    .bind(courtesy_reason.clone())
    .bind(cash_received)
    .bind(change_given)
    .bind(&created_by)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE sales SET idempotency_key = $1, cash_session_id = $2 WHERE id = $3")
        .bind(dto.idempotency_key)
        .bind(cash_session_id)
        .bind(sale.id)
        .execute(&mut *tx)
        .await?;

    // 3. Insertar sale_items y descontar stock de cada uno
    let mut items = Vec::new();
    for line in lines {
        let saved_item = sqlx::query_as::<_, SaleItem>(
            "INSERT INTO sale_items
                 (sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal",
        )
        .bind(sale.id)
        .bind(line.variant_id)
        .bind(line.quantity)
        .bind(line.unit_price)
        .bind(line.unit_cost)
        .bind(line.discount)
        .bind(line.tax)
        .bind(line.subtotal)
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
                unit_cost: line.unit_cost,
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

    // 5. Pago en efectivo -> movimiento de caja del turno bloqueado al inicio
    if cash_net > 0 {
        let movement = cash_repo::create_movement(
            &mut tx,
            cash_session_id,
            CreateMovementDto {
                movement_type: CashMovementType::SaleIn,
                amount: cash_net,
                notes: Some(format!("Venta #{}", sale.id)),
                created_by: Some(created_by.clone()),
                sale_id: Some(sale.id),
            },
        )
        .await?;

        // Defensa adicional: sin movimiento de caja se revierte la venta.
        if movement.is_none() {
            return Err(CreateSaleError::NoOpenCashSession);
        }
    }

    // El detalle por ítem vive en sale_items; este evento conserva el resumen.
    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Sales,
            action: "create".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("sale".to_string()),
            entity_id: Some(sale.id.to_string()),
            summary: format!("Venta #{} registrada", sale.id),
            changes: None,
            metadata: Some(json!({
                "subtotal": sale.subtotal,
                "discount": sale.discount,
                "discount_lines": discount_lines,
                "discount_global": global_discount,
                "tax": sale.tax,
                "total": sale.total,
                "item_count": items.len(),
                "units": items.iter().map(|item| item.quantity as i64).sum::<i64>(),
                "payments": payments
                    .iter()
                    .map(|payment| json!({
                        "method": payment.method,
                        "amount": payment.amount,
                    }))
                    .collect::<Vec<_>>(),
                "customer_id": sale.customer_id,
                "cash_session_id": cash_session_id,
                "idempotency_key": dto.idempotency_key,
                "courtesy": sale.courtesy_reason.is_some(),
                "courtesy_reason": sale.courtesy_reason,
                "cash_received": sale.cash_received,
                "change_given": sale.change_given,
            })),
            error_message: None,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(SaleDetail {
        sale,
        items,
        payments,
        item_names: Vec::new(),
    })
}

pub async fn get_sale_by_id(pool: &PgPool, id: i32) -> Result<Option<SaleDetail>, sqlx::Error> {
    let sale = sqlx::query_as::<_, Sale>(
        r#"SELECT id, customer_id, subtotal, tax, discount, total,
                  cash_received, change_given, status, notes, courtesy_reason, created_by, created_at,
                  voided_at, void_reason, voided_by
           FROM sales WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(sale) = sale else { return Ok(None) };

    let items = sqlx::query_as::<_, SaleItem>(
        "SELECT id, sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal
         FROM sale_items WHERE sale_id = $1 ORDER BY id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let item_names = sqlx::query_as::<_, crate::modules::sales::models::SaleItemName>(
        r#"SELECT si.id AS item_id, p.name AS product_name, v.sku,
                  COALESCE(v.attributes, '{}'::jsonb) AS attributes
           FROM sale_items si
           JOIN product_variants v ON v.id = si.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE si.sale_id = $1 ORDER BY si.id"#,
    )
    .bind(id)
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
        item_names,
    }))
}

pub async fn void_sale(
    pool: &PgPool,
    id: i32,
    dto: VoidSaleDto,
) -> Result<SaleDetail, VoidSaleError> {
    let mut tx = pool.begin().await?;

    let status =
        sqlx::query_scalar::<_, SaleStatus>("SELECT status FROM sales WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(VoidSaleError::NotFound)?;
    if status != SaleStatus::Completed {
        return Err(VoidSaleError::NotCompleted(status));
    }

    let cash_net = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM sale_payments \
         WHERE sale_id = $1 AND method = 'cash'",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    let session_id = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM cash_sessions WHERE status = 'open' FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?;
    if cash_net > 0 && session_id.is_none() {
        return Err(VoidSaleError::CashSessionRequired);
    }

    let items = sqlx::query_as::<_, SaleItem>(
        "SELECT id, sale_id, variant_id, quantity, unit_price, unit_cost, discount, tax, subtotal \
         FROM sale_items WHERE sale_id = $1 ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let mut variant_ids = items.iter().map(|item| item.variant_id).collect::<Vec<_>>();
    variant_ids.sort_unstable();
    variant_ids.dedup();
    if !variant_ids.is_empty() {
        sqlx::query_scalar::<_, i32>(
            "SELECT id FROM product_variants WHERE id = ANY($1) ORDER BY id FOR UPDATE",
        )
        .bind(&variant_ids)
        .fetch_all(&mut *tx)
        .await?;
    }

    let actor = declared_actor(&dto.performed_by);
    let movement_actor = dto
        .performed_by
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("system");
    let label = match dto.target_status {
        SaleStatus::Cancelled => "Anulación",
        SaleStatus::Refunded => "Devolución",
        SaleStatus::Completed => unreachable!("service validates target status"),
    };
    let notes = format!("{label} venta #{id}");
    for item in &items {
        inventory_repo::apply_stock_return(
            &mut tx,
            item.variant_id,
            item.quantity,
            item.unit_cost,
            id,
            &notes,
            movement_actor,
        )
        .await?;
    }

    if cash_net > 0 {
        let session_id = session_id.ok_or(VoidSaleError::CashSessionRequired)?;
        let movement = cash_repo::create_movement(
            &mut tx,
            session_id,
            CreateMovementDto {
                movement_type: CashMovementType::SaleOut,
                amount: cash_net,
                notes: Some(notes.clone()),
                created_by: Some(movement_actor.to_string()),
                sale_id: Some(id),
            },
        )
        .await?;
        if movement.is_none() {
            return Err(VoidSaleError::CashSessionRequired);
        }
    }

    let updated = sqlx::query(
        "UPDATE sales SET status = $2, voided_at = NOW(), void_reason = $3, voided_by = $4 \
         WHERE id = $1 AND status = 'completed'",
    )
    .bind(id)
    .bind(dto.target_status)
    .bind(dto.reason.trim())
    .bind(movement_actor)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(VoidSaleError::NotCompleted(status));
    }

    let action = match dto.target_status {
        SaleStatus::Cancelled => "cancel",
        SaleStatus::Refunded => "refund",
        SaleStatus::Completed => unreachable!("service validates target status"),
    };
    let units = items
        .iter()
        .map(|item| i64::from(item.quantity))
        .sum::<i64>();
    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Sales,
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("sale".to_string()),
            entity_id: Some(id.to_string()),
            summary: format!("{label} de venta #{id}"),
            changes: Some(json!({
                "status": { "from": "completed", "to": dto.target_status }
            })),
            metadata: Some(json!({
                "reason": dto.reason.trim(),
                "cash_refunded": cash_net,
                "units": units,
                "session_id": session_id
            })),
            error_message: None,
        },
    )
    .await?;

    tx.commit().await?;
    get_sale_by_id(pool, id)
        .await?
        .ok_or(VoidSaleError::NotFound)
}

/// Cambia solo `sales.customer_id`; no modifica importes, stock ni caja.
/// Si la asignación ya coincide, no genera una escritura ni un evento de auditoría.
pub async fn assign_customer(
    pool: &PgPool,
    id: i32,
    dto: AssignSaleCustomerDto,
) -> Result<SaleDetail, AssignCustomerError> {
    let actor = declared_actor(&dto.performed_by);
    let mut tx = pool.begin().await?;

    let previous: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT customer_id FROM sales WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AssignCustomerError::SaleNotFound)?;

    let mut new_name: Option<String> = None;
    if let Some(customer_id) = dto.customer_id {
        let row: Option<(String, bool)> =
            sqlx::query_as("SELECT name, is_active FROM customers WHERE id = $1 FOR SHARE")
                .bind(customer_id)
                .fetch_optional(&mut *tx)
                .await?;
        match row {
            None => return Err(AssignCustomerError::CustomerNotFound),
            Some((_, false)) => return Err(AssignCustomerError::CustomerInactive),
            Some((name, true)) => new_name = Some(name),
        }
    }

    if previous == dto.customer_id {
        tx.rollback().await?;
        return get_sale_by_id(pool, id)
            .await?
            .ok_or(AssignCustomerError::SaleNotFound);
    }

    let previous_name: Option<String> = match previous {
        Some(previous_id) => {
            sqlx::query_scalar("SELECT name FROM customers WHERE id = $1")
                .bind(previous_id)
                .fetch_optional(&mut *tx)
                .await?
        }
        None => None,
    };

    sqlx::query("UPDATE sales SET customer_id = $2 WHERE id = $1")
        .bind(id)
        .bind(dto.customer_id)
        .execute(&mut *tx)
        .await?;

    let summary = match &new_name {
        Some(name) => format!("Venta #{id} asignada al cliente {name}"),
        None => format!("Venta #{id} quedó sin cliente"),
    };
    audit_repo::insert_event(
        &mut *tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Sales,
            action: "assign_customer".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("sale".to_string()),
            entity_id: Some(id.to_string()),
            summary,
            changes: Some(json!({
                "customer_id": { "from": previous, "to": dto.customer_id }
            })),
            metadata: Some(json!({
                "from_name": previous_name,
                "to_name": new_name,
            })),
            error_message: None,
        },
    )
    .await?;

    tx.commit().await?;
    get_sale_by_id(pool, id)
        .await?
        .ok_or(AssignCustomerError::SaleNotFound)
}

pub async fn list_sales(
    pool: &PgPool,
    filter: &SaleFilterDto,
) -> Result<(Vec<Sale>, i64, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as::<_, Sale>(
        r#"SELECT id, customer_id, subtotal, tax, discount, total,
                  cash_received, change_given, status, notes, courtesy_reason, created_by, created_at,
                  voided_at, void_reason, voided_by
           FROM sales
           WHERE ($1::TEXT IS NULL OR status::TEXT = $1)
             AND ($2::INT  IS NULL OR customer_id = $2)
             AND ($3::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE >= $3)
             AND ($4::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE <= $4)
             AND ($5::INT IS NULL OR id = $5)
             AND ($6::TEXT IS NULL OR EXISTS
                  (SELECT 1 FROM sale_payments sp WHERE sp.sale_id = sales.id AND sp.method::TEXT = $6))
             AND ($7::INT IS NULL OR cash_session_id = $7)
           ORDER BY created_at DESC
           LIMIT $8 OFFSET $9"#,
    )
    .bind(filter.status.as_deref())
    .bind(filter.customer_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(filter.sale_id)
    .bind(filter.payment_method.as_deref())
    .bind(filter.cash_session_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)::BIGINT FROM sales
           WHERE ($1::TEXT IS NULL OR status::TEXT = $1)
             AND ($2::INT  IS NULL OR customer_id = $2)
             AND ($3::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE >= $3)
             AND ($4::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE <= $4)
             AND ($5::INT IS NULL OR id = $5)
             AND ($6::TEXT IS NULL OR EXISTS
                  (SELECT 1 FROM sale_payments sp WHERE sp.sale_id = sales.id AND sp.method::TEXT = $6))
             AND ($7::INT IS NULL OR cash_session_id = $7)"#,
    )
    .bind(filter.status.as_deref())
    .bind(filter.customer_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(filter.sale_id)
    .bind(filter.payment_method.as_deref())
    .bind(filter.cash_session_id)
    .fetch_one(pool)
    .await?;

    let total_amount = sqlx::query_scalar::<_, i64>(
        r#"SELECT COALESCE(SUM(total), 0)::BIGINT FROM sales
           WHERE status = 'completed'
             AND ($1::TEXT IS NULL OR status::TEXT = $1)
             AND ($2::INT IS NULL OR customer_id = $2)
             AND ($3::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE >= $3)
             AND ($4::DATE IS NULL OR
                  (created_at AT TIME ZONE (SELECT timezone FROM app_settings WHERE id = 1))::DATE <= $4)
             AND ($5::INT IS NULL OR id = $5)
             AND ($6::TEXT IS NULL OR EXISTS
                  (SELECT 1 FROM sale_payments sp WHERE sp.sale_id = sales.id AND sp.method::TEXT = $6))
             AND ($7::INT IS NULL OR cash_session_id = $7)"#,
    )
    .bind(filter.status.as_deref())
    .bind(filter.customer_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(filter.sale_id)
    .bind(filter.payment_method.as_deref())
    .bind(filter.cash_session_id)
    .fetch_one(pool)
    .await?;

    Ok((rows, total, total_amount))
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

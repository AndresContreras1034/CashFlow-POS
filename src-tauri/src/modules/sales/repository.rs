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

/// Errores de negocio propios de la confirmación de venta —
/// se distinguen de sqlx::Error para poder dar mensajes claros en el service.
pub enum CreateSaleError {
    Db(sqlx::Error),
    NoOpenCashSession,
    PaymentMismatch,
    InsufficientStock {
        variant_id: i32,
        available: i32,
        requested: i32,
    },
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

    // 1. Precio server-side por cada ítem — nunca se confía en el precio del cliente
    struct LineItem {
        variant_id: i32,
        quantity: i32,
        unit_price: i64,
        discount: i64,
        subtotal: i64,
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
        let line_subtotal = price * item.quantity as i64 - discount;

        subtotal_total += line_subtotal;
        lines.push(LineItem {
            variant_id: item.variant_id,
            quantity: item.quantity,
            unit_price: price,
            discount,
            subtotal: line_subtotal,
        });
    }

    let total = subtotal_total - global_discount;
    let payments_total: i64 = dto.payments.iter().map(|p| p.amount).sum();

    if payments_total != total {
        return Err(CreateSaleError::PaymentMismatch);
    }

    // 2. Insertar sale
    let sale = sqlx::query_as!(
        Sale,
        r#"INSERT INTO sales (customer_id, subtotal, tax, discount, total, status, notes, created_by)
           VALUES ($1, $2, 0, $3, $4, 'completed', $5, $6)
           RETURNING id, customer_id, subtotal, tax, discount, total,
                     status AS "status: SaleStatus", notes, created_by, created_at"#,
        dto.customer_id,
        subtotal_total,
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
             VALUES ($1, $2, $3, $4, $5, 0, $6)
             RETURNING *",
            sale.id,
            line.variant_id,
            line.quantity,
            line.unit_price,
            line.discount,
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

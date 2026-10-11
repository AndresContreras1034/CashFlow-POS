use chrono::NaiveDate;
use serde_json::{json, Value as JsonValue};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditEvent, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::debts::{
    dto::{
        CreateDebtDto, DebtFilterDto, DebtSortBy, RegisterPaymentDto, SortDir, UpdateDebtDto,
        VoidDebtDto, VoidPaymentDto,
    },
    models::{
        Debt, DebtCategory, DebtCategoryStat, DebtNature, DebtOverview, DebtPayment,
        DebtPaymentMethod, DebtStatus, DebtSummary, DebtUrgency,
    },
};

/// Errores de negocio de las operaciones de deudas. Se distinguen de
/// `sqlx::Error` para que el service (Etapa 2) pueda dar mensajes claros,
/// igual que `CloseSessionError` y `CreateSaleError`.
#[derive(Debug)]
pub enum DebtError {
    Db(sqlx::Error),
    NotFound,
    PaymentNotFound,
    /// La deuda está anulada y no admite cambios ni pagos.
    DebtVoided,
    DebtAlreadyVoided,
    ReasonRequired,
    InvalidAmount,
    IssuedInFuture,
    DueBeforeIssue,
    /// Para anular la deuda hay que reversar antes sus pagos vigentes.
    HasLivePayments,
    AmountChangeWithPayments,
    IssuedAfterFirstPayment,
    PaymentInFuture,
    PaymentBeforeIssue,
    Overpayment {
        balance: i64,
        requested: i64,
    },
    PaymentAlreadyVoided,
    /// La clave de idempotencia ya existe con datos distintos.
    IdempotencyConflict,
}

impl From<sqlx::Error> for DebtError {
    fn from(e: sqlx::Error) -> Self {
        DebtError::Db(e)
    }
}

// ============================================================
// HELPERS
// ============================================================

/// «Hoy» según la zona horaria de Ajustes (función SQL `debt_today()`, la
/// misma que usa la vista `debt_overview`).
pub async fn today(conn: &mut PgConnection) -> Result<NaiveDate, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT debt_today() AS "today!""#)
        .fetch_one(&mut *conn)
        .await
}

fn required_reason(reason: &str) -> Result<String, DebtError> {
    let reason = reason.trim();
    if reason.is_empty() {
        Err(DebtError::ReasonRequired)
    } else {
        Ok(reason.to_string())
    }
}

fn same_payload_as_create(debt: &Debt, dto: &CreateDebtDto) -> bool {
    debt.creditor_name == dto.creditor_name
        && debt.category == dto.category
        && debt.subcategory == dto.subcategory
        && debt.nature == dto.nature
        && debt.concept == dto.concept
        && debt.original_amount == dto.original_amount
        && debt.issued_on == dto.issued_on
        && debt.due_on == dto.due_on
        && debt.document_ref == dto.document_ref
        && debt.notes == dto.notes
}

fn same_payload_as_update(debt: &Debt, dto: &UpdateDebtDto) -> bool {
    debt.creditor_name == dto.creditor_name
        && debt.category == dto.category
        && debt.subcategory == dto.subcategory
        && debt.nature == dto.nature
        && debt.concept == dto.concept
        && debt.original_amount == dto.original_amount
        && debt.issued_on == dto.issued_on
        && debt.due_on == dto.due_on
        && debt.document_ref == dto.document_ref
        && debt.notes == dto.notes
}

fn same_payload_as_payment(payment: &DebtPayment, debt_id: i32, dto: &RegisterPaymentDto) -> bool {
    payment.debt_id == debt_id
        && payment.amount == dto.amount
        && payment.paid_on == dto.paid_on
        && payment.method == dto.method
        && payment.reference == dto.reference
        && payment.notes == dto.notes
}

/// Bloquea la fila de la deuda hasta el fin de la transacción. Toda
/// operación que cambie una deuda o sus pagos empieza por aquí (orden de
/// bloqueo fijo: primero `debts`, después `debt_payments`).
async fn lock_debt(conn: &mut PgConnection, id: i32) -> Result<Option<Debt>, sqlx::Error> {
    sqlx::query_as!(
        Debt,
        r#"SELECT id, creditor_name,
                  category AS "category: DebtCategory",
                  subcategory,
                  nature AS "nature: DebtNature",
                  concept, original_amount, issued_on, due_on, document_ref, notes,
                  idempotency_key, voided_at, voided_by, void_reason,
                  created_by, created_at, updated_at
           FROM debts WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
}

struct LivePayments {
    count: i64,
    paid: i64,
    first_paid_on: Option<NaiveDate>,
}

async fn live_payments(conn: &mut PgConnection, debt_id: i32) -> Result<LivePayments, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT COUNT(*) AS "count!",
                  COALESCE(SUM(amount), 0)::BIGINT AS "paid!",
                  MIN(paid_on) AS first_paid_on
           FROM debt_payments
           WHERE debt_id = $1 AND voided_at IS NULL"#,
        debt_id
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(LivePayments {
        count: row.count,
        paid: row.paid,
        first_paid_on: row.first_paid_on,
    })
}

#[allow(clippy::too_many_arguments)]
async fn record_event(
    conn: &mut PgConnection,
    actor: Option<String>,
    action: &str,
    entity_type: &str,
    entity_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> Result<(), sqlx::Error> {
    audit_repo::insert_event(
        conn,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Debts,
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some(entity_type.to_string()),
            entity_id: Some(entity_id.to_string()),
            summary,
            changes,
            metadata: Some(metadata),
            error_message: None,
        },
    )
    .await?;
    Ok(())
}

// ============================================================
// DEUDAS
// ============================================================

/// Registra una deuda y su evento de auditoría en una sola transacción.
///
/// Idempotente: repetir la llamada con la misma `idempotency_key` y los
/// mismos datos devuelve la deuda existente, sin crear otra ni otro evento.
/// La misma clave con datos distintos da `IdempotencyConflict`.
pub async fn create_debt(pool: &PgPool, dto: CreateDebtDto) -> Result<Debt, DebtError> {
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await?;

    if dto.original_amount <= 0 {
        return Err(DebtError::InvalidAmount);
    }
    if dto.issued_on > today(&mut tx).await? {
        return Err(DebtError::IssuedInFuture);
    }
    if dto.due_on.is_some_and(|due| due < dto.issued_on) {
        return Err(DebtError::DueBeforeIssue);
    }

    let inserted = sqlx::query_as!(
        Debt,
        r#"INSERT INTO debts
               (creditor_name, category, subcategory, nature, concept,
                original_amount, issued_on, due_on, document_ref, notes,
                idempotency_key, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           ON CONFLICT (idempotency_key) DO NOTHING
           RETURNING id, creditor_name,
                     category AS "category: DebtCategory",
                     subcategory,
                     nature AS "nature: DebtNature",
                     concept, original_amount, issued_on, due_on, document_ref, notes,
                     idempotency_key, voided_at, voided_by, void_reason,
                     created_by, created_at, updated_at"#,
        dto.creditor_name,
        dto.category as DebtCategory,
        dto.subcategory,
        dto.nature as DebtNature,
        dto.concept,
        dto.original_amount,
        dto.issued_on,
        dto.due_on,
        dto.document_ref,
        dto.notes,
        dto.idempotency_key,
        created_by
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(debt) = inserted else {
        // Repetición: la clave ya existía. Se devuelve la deuda original.
        let existing = sqlx::query_as!(
            Debt,
            r#"SELECT id, creditor_name,
                      category AS "category: DebtCategory",
                      subcategory,
                      nature AS "nature: DebtNature",
                      concept, original_amount, issued_on, due_on, document_ref, notes,
                      idempotency_key, voided_at, voided_by, void_reason,
                      created_by, created_at, updated_at
               FROM debts WHERE idempotency_key = $1"#,
            dto.idempotency_key
        )
        .fetch_one(&mut *tx)
        .await?;

        return if same_payload_as_create(&existing, &dto) {
            Ok(existing)
        } else {
            Err(DebtError::IdempotencyConflict)
        };
    };

    record_event(
        &mut tx,
        actor,
        "create",
        "debt",
        debt.id,
        format!("Deuda #{} registrada: {}", debt.id, debt.concept),
        None,
        json!({
            "creditor_name": debt.creditor_name,
            "category": debt.category,
            "subcategory": debt.subcategory,
            "nature": debt.nature,
            "original_amount": debt.original_amount,
            "issued_on": debt.issued_on,
            "due_on": debt.due_on,
            "document_ref": debt.document_ref,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(debt)
}

/// Edita los campos permitidos. Si no cambia nada, no escribe ni audita.
///
/// Con pagos vigentes: el importe no se puede cambiar y la emisión no puede
/// pasar del primer pago.
pub async fn update_debt(pool: &PgPool, id: i32, dto: UpdateDebtDto) -> Result<Debt, DebtError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;

    let before = lock_debt(&mut tx, id).await?.ok_or(DebtError::NotFound)?;
    if before.voided_at.is_some() {
        return Err(DebtError::DebtVoided);
    }

    if dto.original_amount <= 0 {
        return Err(DebtError::InvalidAmount);
    }
    if dto.issued_on > today(&mut tx).await? {
        return Err(DebtError::IssuedInFuture);
    }
    if dto.due_on.is_some_and(|due| due < dto.issued_on) {
        return Err(DebtError::DueBeforeIssue);
    }

    let live = live_payments(&mut tx, id).await?;
    if live.count > 0 {
        if dto.original_amount != before.original_amount {
            return Err(DebtError::AmountChangeWithPayments);
        }
        if live
            .first_paid_on
            .is_some_and(|first| dto.issued_on > first)
        {
            return Err(DebtError::IssuedAfterFirstPayment);
        }
    }

    if same_payload_as_update(&before, &dto) {
        return Ok(before);
    }

    let after = sqlx::query_as!(
        Debt,
        r#"UPDATE debts
           SET creditor_name   = $2,
               category        = $3,
               subcategory     = $4,
               nature          = $5,
               concept         = $6,
               original_amount = $7,
               issued_on       = $8,
               due_on          = $9,
               document_ref    = $10,
               notes           = $11
           WHERE id = $1
           RETURNING id, creditor_name,
                     category AS "category: DebtCategory",
                     subcategory,
                     nature AS "nature: DebtNature",
                     concept, original_amount, issued_on, due_on, document_ref, notes,
                     idempotency_key, voided_at, voided_by, void_reason,
                     created_by, created_at, updated_at"#,
        id,
        dto.creditor_name,
        dto.category as DebtCategory,
        dto.subcategory,
        dto.nature as DebtNature,
        dto.concept,
        dto.original_amount,
        dto.issued_on,
        dto.due_on,
        dto.document_ref,
        dto.notes
    )
    .fetch_one(&mut *tx)
    .await?;

    let changes = changed_fields(&before, &after, &["id", "updated_at"]);

    record_event(
        &mut tx,
        actor,
        "update",
        "debt",
        after.id,
        format!("Deuda #{} actualizada", after.id),
        Some(JsonValue::Object(changes)),
        json!({ "creditor_name": after.creditor_name }),
    )
    .await?;

    tx.commit().await?;
    Ok(after)
}

/// Anula la deuda. Exige motivo y que no queden pagos vigentes: los pagos
/// se reversan primero, uno a uno (no hay anulación en cascada).
pub async fn void_debt(pool: &PgPool, id: i32, dto: VoidDebtDto) -> Result<Debt, DebtError> {
    let reason = required_reason(&dto.reason)?;
    let actor = declared_actor(&dto.voided_by);
    let mut tx = pool.begin().await?;

    let debt = lock_debt(&mut tx, id).await?.ok_or(DebtError::NotFound)?;
    if debt.voided_at.is_some() {
        return Err(DebtError::DebtAlreadyVoided);
    }
    if live_payments(&mut tx, id).await?.count > 0 {
        return Err(DebtError::HasLivePayments);
    }

    let voided = sqlx::query_as!(
        Debt,
        r#"UPDATE debts
           SET voided_at = NOW(), voided_by = $2, void_reason = $3
           WHERE id = $1
           RETURNING id, creditor_name,
                     category AS "category: DebtCategory",
                     subcategory,
                     nature AS "nature: DebtNature",
                     concept, original_amount, issued_on, due_on, document_ref, notes,
                     idempotency_key, voided_at, voided_by, void_reason,
                     created_by, created_at, updated_at"#,
        id,
        actor,
        reason
    )
    .fetch_one(&mut *tx)
    .await?;

    record_event(
        &mut tx,
        actor,
        "void",
        "debt",
        voided.id,
        format!("Deuda #{} anulada", voided.id),
        Some(json!({ "status": { "from": "active", "to": "voided" } })),
        json!({
            "reason": voided.void_reason,
            "creditor_name": voided.creditor_name,
            "original_amount": voided.original_amount,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(voided)
}

// ============================================================
// PAGOS
// ============================================================

/// Registra un pago o abono. Todo ocurre bajo el bloqueo de la deuda:
/// validación de saldo, inserción y evento de auditoría.
///
/// Idempotente por `idempotency_key`, con la misma regla que `create_debt`.
/// La repetición se resuelve ANTES de validar el saldo: reintentar un pago
/// que ya se aplicó no debe fallar por «sobrepago».
pub async fn register_payment(
    pool: &PgPool,
    debt_id: i32,
    dto: RegisterPaymentDto,
) -> Result<DebtPayment, DebtError> {
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await?;

    let debt = lock_debt(&mut tx, debt_id)
        .await?
        .ok_or(DebtError::NotFound)?;

    // 1. Repetición del mismo envío.
    let replay = sqlx::query_as!(
        DebtPayment,
        r#"SELECT id, debt_id, amount, paid_on,
                  method AS "method: DebtPaymentMethod",
                  reference, notes, idempotency_key,
                  voided_at, voided_by, void_reason, created_by, created_at
           FROM debt_payments WHERE idempotency_key = $1"#,
        dto.idempotency_key
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(existing) = replay {
        return if same_payload_as_payment(&existing, debt_id, &dto) {
            Ok(existing)
        } else {
            Err(DebtError::IdempotencyConflict)
        };
    }

    // 2. Reglas de negocio.
    if debt.voided_at.is_some() {
        return Err(DebtError::DebtVoided);
    }
    if dto.amount <= 0 {
        return Err(DebtError::InvalidAmount);
    }
    if dto.paid_on > today(&mut tx).await? {
        return Err(DebtError::PaymentInFuture);
    }
    if dto.paid_on < debt.issued_on {
        return Err(DebtError::PaymentBeforeIssue);
    }

    let paid_before = live_payments(&mut tx, debt_id).await?.paid;
    let balance = debt.original_amount - paid_before;
    if dto.amount > balance {
        return Err(DebtError::Overpayment {
            balance,
            requested: dto.amount,
        });
    }

    // 3. Inserción.
    let inserted = sqlx::query_as!(
        DebtPayment,
        r#"INSERT INTO debt_payments
               (debt_id, amount, paid_on, method, reference, notes,
                idempotency_key, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (idempotency_key) DO NOTHING
           RETURNING id, debt_id, amount, paid_on,
                     method AS "method: DebtPaymentMethod",
                     reference, notes, idempotency_key,
                     voided_at, voided_by, void_reason, created_by, created_at"#,
        debt_id,
        dto.amount,
        dto.paid_on,
        dto.method as DebtPaymentMethod,
        dto.reference,
        dto.notes,
        dto.idempotency_key,
        created_by
    )
    .fetch_optional(&mut *tx)
    .await?;

    // Solo ocurre si la misma clave se usó para un pago de OTRA deuda.
    let payment = inserted.ok_or(DebtError::IdempotencyConflict)?;

    record_event(
        &mut tx,
        actor,
        "create",
        "debt_payment",
        payment.id,
        format!("Pago registrado en la deuda #{}", debt_id),
        None,
        json!({
            "debt_id": debt_id,
            "amount": payment.amount,
            "paid_on": payment.paid_on,
            "method": payment.method,
            "reference": payment.reference,
            "balance_after": balance - payment.amount,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(payment)
}

/// Anula (reversa) un pago. El pago permanece con su motivo; el saldo se
/// recupera porque la vista solo suma los pagos vigentes.
pub async fn void_payment(
    pool: &PgPool,
    payment_id: i32,
    dto: VoidPaymentDto,
) -> Result<DebtPayment, DebtError> {
    let reason = required_reason(&dto.reason)?;
    let actor = declared_actor(&dto.voided_by);
    let mut tx = pool.begin().await?;

    // Mismo orden de bloqueo que el resto: primero la deuda, luego el pago.
    let debt_id = sqlx::query_scalar!(
        "SELECT debt_id FROM debt_payments WHERE id = $1",
        payment_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DebtError::PaymentNotFound)?;

    let debt = lock_debt(&mut tx, debt_id)
        .await?
        .ok_or(DebtError::NotFound)?;

    let current = sqlx::query_as!(
        DebtPayment,
        r#"SELECT id, debt_id, amount, paid_on,
                  method AS "method: DebtPaymentMethod",
                  reference, notes, idempotency_key,
                  voided_at, voided_by, void_reason, created_by, created_at
           FROM debt_payments WHERE id = $1 FOR UPDATE"#,
        payment_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if current.voided_at.is_some() {
        return Err(DebtError::PaymentAlreadyVoided);
    }

    let voided = sqlx::query_as!(
        DebtPayment,
        r#"UPDATE debt_payments
           SET voided_at = NOW(), voided_by = $2, void_reason = $3
           WHERE id = $1
           RETURNING id, debt_id, amount, paid_on,
                     method AS "method: DebtPaymentMethod",
                     reference, notes, idempotency_key,
                     voided_at, voided_by, void_reason, created_by, created_at"#,
        payment_id,
        actor,
        reason
    )
    .fetch_one(&mut *tx)
    .await?;

    let paid_after = live_payments(&mut tx, debt_id).await?.paid;

    record_event(
        &mut tx,
        actor,
        "void",
        "debt_payment",
        voided.id,
        format!("Pago #{} de la deuda #{} anulado", voided.id, debt_id),
        Some(json!({ "status": { "from": "active", "to": "voided" } })),
        json!({
            "debt_id": debt_id,
            "amount": voided.amount,
            "reason": voided.void_reason,
            "balance_after": debt.original_amount - paid_after,
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(voided)
}

// ============================================================
// LECTURAS
// ============================================================

/// La deuda con saldo, estado y urgencia calculados por la base de datos.
pub async fn get_overview(pool: &PgPool, id: i32) -> Result<Option<DebtOverview>, sqlx::Error> {
    sqlx::query_as!(
        DebtOverview,
        r#"SELECT
               id                AS "id!",
               creditor_name     AS "creditor_name!",
               category          AS "category!: DebtCategory",
               subcategory,
               nature            AS "nature!: DebtNature",
               concept           AS "concept!",
               original_amount   AS "original_amount!",
               issued_on         AS "issued_on!",
               due_on,
               document_ref,
               notes,
               voided_at,
               voided_by,
               void_reason,
               created_by        AS "created_by!",
               created_at        AS "created_at!",
               updated_at        AS "updated_at!",
               paid_amount       AS "paid_amount!",
               balance           AS "balance!",
               payment_count     AS "payment_count!",
               last_paid_on,
               today             AS "today!",
               days_until_due,
               status            AS "status!: DebtStatus",
               urgency           AS "urgency!: DebtUrgency"
           FROM debt_overview
           WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Todos los pagos de la deuda, vigentes y anulados, del más antiguo al más
/// reciente (historial completo).
pub async fn list_payments(pool: &PgPool, debt_id: i32) -> Result<Vec<DebtPayment>, sqlx::Error> {
    sqlx::query_as!(
        DebtPayment,
        r#"SELECT id, debt_id, amount, paid_on,
                  method AS "method: DebtPaymentMethod",
                  reference, notes, idempotency_key,
                  voided_at, voided_by, void_reason, created_by, created_at
           FROM debt_payments
           WHERE debt_id = $1
           ORDER BY created_at ASC, id ASC"#,
        debt_id
    )
    .fetch_all(pool)
    .await
}

/// Decimales de la moneda según Ajustes (0 o 2). Los importes se validan
/// contra esta precisión en el service; aquí solo se lee.
pub async fn currency_decimals(pool: &PgPool) -> Result<u8, sqlx::Error> {
    let decimals = sqlx::query_scalar!("SELECT currency_decimals FROM app_settings WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(decimals.clamp(0, 2) as u8)
}

// ============================================================
// LISTADO, RESUMEN Y ESTADÍSTICAS
// ============================================================
//
// Estas consultas usan `QueryBuilder` en lugar de los macros `query!`
// (que sí usa el resto del repository) por dos razones:
//   * el listado combina ~15 filtros opcionales y 5 columnas de orden en
//     ambos sentidos: con macros serían decenas de consultas casi idénticas;
//   * así el WHERE se define UNA vez y lo comparten listado, total, resumen
//     y estadísticas, y sus cifras no pueden divergir.
// Todo valor del usuario va como parámetro (`push_bind`). El único texto
// que se concatena (columna y sentido del ORDER BY, y las listas de estados)
// sale de enums y constantes de este archivo, nunca del cliente.

/// Estados con saldo por pagar.
const ACTIVE_STATUSES: &str = "o.status IN ('pending', 'partially_paid', 'overdue')";
/// Próximas a vencer = urgencia `alert` o `due_today` (el umbral de días
/// vive solo en la vista `debt_overview`).
const DUE_SOON_URGENCIES: &str = "o.urgency IN ('alert', 'due_today')";

fn clean(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Patrón ILIKE para `search`, escapando `\`, `%` y `_` para que el texto
/// del usuario se busque literalmente.
pub fn search_pattern(search: &Option<String>) -> Option<String> {
    let text = clean(search)?;
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, f: &DebtFilterDto) {
    qb.push(" WHERE TRUE");

    if let Some(pattern) = search_pattern(&f.search) {
        qb.push(" AND (o.creditor_name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR o.concept ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR o.document_ref ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(category) = f.category {
        qb.push(" AND o.category = ").push_bind(category);
    }
    if let Some(nature) = f.nature {
        qb.push(" AND o.nature = ").push_bind(nature);
    }
    if let Some(status) = f.status {
        qb.push(" AND o.status = ").push_bind(status);
    }
    if let Some(urgencies) = f.urgencies.as_ref().filter(|list| !list.is_empty()) {
        qb.push(" AND o.urgency IN (");
        let mut list = qb.separated(", ");
        for urgency in urgencies {
            list.push_bind(*urgency);
        }
        list.push_unseparated(")");
    }
    if let Some(date) = f.issued_from {
        qb.push(" AND o.issued_on >= ").push_bind(date);
    }
    if let Some(date) = f.issued_to {
        qb.push(" AND o.issued_on <= ").push_bind(date);
    }
    if let Some(date) = f.due_from {
        qb.push(" AND o.due_on >= ").push_bind(date);
    }
    if let Some(date) = f.due_to {
        qb.push(" AND o.due_on <= ").push_bind(date);
    }
    if f.paid_from.is_some() || f.paid_to.is_some() {
        qb.push(
            " AND EXISTS (SELECT 1 FROM debt_payments p
                          WHERE p.debt_id = o.id AND p.voided_at IS NULL",
        );
        if let Some(date) = f.paid_from {
            qb.push(" AND p.paid_on >= ").push_bind(date);
        }
        if let Some(date) = f.paid_to {
            qb.push(" AND p.paid_on <= ").push_bind(date);
        }
        qb.push(")");
    }
}

fn order_clause(sort_by: DebtSortBy, sort_dir: SortDir) -> String {
    let column = match sort_by {
        DebtSortBy::Amount => "o.original_amount",
        DebtSortBy::IssuedOn => "o.issued_on",
        DebtSortBy::CreatedAt => "o.created_at",
        DebtSortBy::DueOn => "o.due_on",
        DebtSortBy::Balance => "o.balance",
    };
    let direction = match sort_dir {
        SortDir::Asc => "ASC",
        SortDir::Desc => "DESC",
    };
    // Las deudas sin vencimiento van siempre al final; `id` desempata.
    format!(" ORDER BY {column} {direction} NULLS LAST, o.id DESC")
}

/// Listado con filtros, orden y paginación. Devuelve la página y el total
/// de filas que cumplen el filtro.
pub async fn list_debts(
    pool: &PgPool,
    filter: &DebtFilterDto,
) -> Result<(Vec<DebtOverview>, i64), sqlx::Error> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(page_size);

    let mut rows_query = QueryBuilder::<Postgres>::new("SELECT o.* FROM debt_overview o");
    push_filters(&mut rows_query, filter);
    rows_query.push(order_clause(
        filter.sort_by.unwrap_or_default(),
        filter.sort_dir.unwrap_or_default(),
    ));
    rows_query
        .push(" LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows = rows_query
        .build_query_as::<DebtOverview>()
        .fetch_all(pool)
        .await?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM debt_overview o");
    push_filters(&mut count_query, filter);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    Ok((rows, total))
}

/// Totales del panel sobre las deudas que cumplen el filtro (orden y
/// paginación se ignoran).
pub async fn summary(pool: &PgPool, filter: &DebtFilterDto) -> Result<DebtSummary, sqlx::Error> {
    let mut qb = QueryBuilder::<Postgres>::new(format!(
        "SELECT
             COALESCE(SUM(o.balance) FILTER (WHERE {ACTIVE_STATUSES}), 0)::BIGINT
                 AS pending_total,
             COALESCE(SUM(o.balance) FILTER (WHERE o.status = 'overdue'), 0)::BIGINT
                 AS overdue_total,
             COALESCE(SUM(o.balance) FILTER (WHERE {DUE_SOON_URGENCIES}), 0)::BIGINT
                 AS due_soon_total,
             (COUNT(*) FILTER (WHERE {ACTIVE_STATUSES}))::BIGINT AS active_count,
             (COUNT(*) FILTER (WHERE o.status = 'overdue'))::BIGINT AS overdue_count,
             (COUNT(*) FILTER (WHERE {DUE_SOON_URGENCIES}))::BIGINT AS due_soon_count
         FROM debt_overview o"
    ));
    push_filters(&mut qb, filter);
    qb.build_query_as::<DebtSummary>().fetch_one(pool).await
}

/// Totales por (categoría, estado) sobre las deudas que cumplen el filtro.
pub async fn category_stats(
    pool: &PgPool,
    filter: &DebtFilterDto,
) -> Result<Vec<DebtCategoryStat>, sqlx::Error> {
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT o.category, o.status,
                COUNT(*)::BIGINT                                   AS count,
                COALESCE(SUM(o.original_amount), 0)::BIGINT        AS original_total,
                COALESCE(SUM(o.paid_amount), 0)::BIGINT            AS paid_total,
                COALESCE(SUM(o.balance) FILTER (WHERE o.status <> 'voided'), 0)::BIGINT
                                                                   AS balance_total
         FROM debt_overview o",
    );
    push_filters(&mut qb, filter);
    qb.push(" GROUP BY o.category, o.status ORDER BY o.category, o.status");
    qb.build_query_as::<DebtCategoryStat>()
        .fetch_all(pool)
        .await
}

// ============================================================
// HISTORIAL
// ============================================================

/// Tope de eventos del historial de una deuda.
const HISTORY_LIMIT: i64 = 500;

/// Historial de modificaciones y anulaciones de una deuda, leído de
/// `audit_events` (única fuente de verdad; no hay tabla de historial propia).
/// Incluye los eventos de sus pagos y excluye los fallos técnicos. Del más
/// reciente al más antiguo.
pub async fn list_history(pool: &PgPool, debt_id: i32) -> Result<Vec<AuditEvent>, sqlx::Error> {
    sqlx::query_as!(
        AuditEvent,
        r#"SELECT
               id, created_at, correlation_id,
               category AS "category: AuditCategory",
               module   AS "module: AuditModule",
               action,
               outcome  AS "outcome: AuditOutcome",
               actor, entity_type, entity_id, summary,
               changes, metadata, error_message
           FROM audit_events
           WHERE module = 'debts'
             AND outcome = 'success'
             AND (   (entity_type = 'debt' AND entity_id = $1)
                  OR (entity_type = 'debt_payment' AND metadata->>'debt_id' = $1))
           ORDER BY created_at DESC, id DESC
           LIMIT $2"#,
        debt_id.to_string(),
        HISTORY_LIMIT
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_pattern_ignores_blank_text() {
        assert_eq!(search_pattern(&None), None);
        assert_eq!(search_pattern(&Some("   ".to_string())), None);
    }

    #[test]
    fn search_pattern_escapes_like_wildcards() {
        assert_eq!(
            search_pattern(&Some(" 50%_off ".to_string())),
            Some("%50\\%\\_off%".to_string())
        );
        assert_eq!(
            search_pattern(&Some("a\\b".to_string())),
            Some("%a\\\\b%".to_string())
        );
    }

    #[test]
    fn order_clause_comes_only_from_the_closed_lists() {
        assert_eq!(
            order_clause(DebtSortBy::DueOn, SortDir::Asc),
            " ORDER BY o.due_on ASC NULLS LAST, o.id DESC"
        );
        assert_eq!(
            order_clause(DebtSortBy::Balance, SortDir::Desc),
            " ORDER BY o.balance DESC NULLS LAST, o.id DESC"
        );
    }
}
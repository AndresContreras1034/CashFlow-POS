//! Pruebas de integración del módulo de deudas (Etapa 1: persistencia).
//!
//! Solo usan `debts` y la tabla de auditoría; no tocan caja, ventas ni
//! inventario. Cada prueba corre en una base temporal creada por
//! `#[sqlx::test]` con todas las migraciones aplicadas.

use chrono::{Duration, NaiveDate};
use pos_lib::modules::debts::{
    dto::{CreateDebtDto, RegisterPaymentDto, UpdateDebtDto, VoidDebtDto, VoidPaymentDto},
    models::{DebtCategory, DebtNature, DebtPaymentMethod, DebtStatus, DebtUrgency},
    repository::{self as repo, DebtError},
};
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================
// HELPERS
// ============================================================

async fn today(pool: &PgPool) -> NaiveDate {
    let mut conn = pool.acquire().await.unwrap();
    repo::today(&mut conn).await.unwrap()
}

fn debt_dto(issued_on: NaiveDate, due_on: Option<NaiveDate>, amount: i64) -> CreateDebtDto {
    CreateDebtDto {
        idempotency_key: Uuid::new_v4(),
        creditor_name: "Ferretería Central".to_string(),
        category: DebtCategory::Maintenance,
        subcategory: Some("Reparaciones".to_string()),
        nature: DebtNature::Operating,
        concept: "Reparación del techo".to_string(),
        original_amount: amount,
        issued_on,
        due_on,
        document_ref: Some("FAC-001".to_string()),
        notes: None,
        created_by: Some("Ana".to_string()),
    }
}

fn update_dto_from(d: &CreateDebtDto) -> UpdateDebtDto {
    UpdateDebtDto {
        creditor_name: d.creditor_name.clone(),
        category: d.category,
        subcategory: d.subcategory.clone(),
        nature: d.nature,
        concept: d.concept.clone(),
        original_amount: d.original_amount,
        issued_on: d.issued_on,
        due_on: d.due_on,
        document_ref: d.document_ref.clone(),
        notes: d.notes.clone(),
        updated_by: Some("Luis".to_string()),
    }
}

fn pay_dto(amount: i64, paid_on: NaiveDate) -> RegisterPaymentDto {
    RegisterPaymentDto {
        idempotency_key: Uuid::new_v4(),
        amount,
        paid_on,
        method: DebtPaymentMethod::Transfer,
        reference: Some("TRX-9".to_string()),
        notes: None,
        created_by: Some("Ana".to_string()),
    }
}

fn void_dto(reason: &str) -> VoidDebtDto {
    VoidDebtDto {
        reason: reason.to_string(),
        voided_by: Some("Ana".to_string()),
    }
}

fn void_pay_dto(reason: &str) -> VoidPaymentDto {
    VoidPaymentDto {
        reason: reason.to_string(),
        voided_by: Some("Ana".to_string()),
    }
}

/// Crea una deuda emitida hace 30 días con vencimiento relativo a «hoy».
async fn create_with_due(pool: &PgPool, due_offset: Option<i64>, amount: i64) -> i32 {
    let t = today(pool).await;
    let dto = debt_dto(
        t - Duration::days(30),
        due_offset.map(|d| t + Duration::days(d)),
        amount,
    );
    repo::create_debt(pool, dto).await.unwrap().id
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn debt_audit_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE module::text = 'debts'")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Hace que todo insert en audit_events falle (simula auditoría caída).
async fn make_audit_fail(pool: &PgPool) {
    sqlx::query(
        r#"CREATE FUNCTION fail_audit_insert() RETURNS trigger AS $$
           BEGIN RAISE EXCEPTION 'audit caido'; END;
           $$ LANGUAGE plpgsql"#,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER trg_fail_audit_insert BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION fail_audit_insert()",
    )
    .execute(pool)
    .await
    .unwrap();
}

fn constraint_of(err: sqlx::Error) -> String {
    err.as_database_error()
        .and_then(|e| e.constraint().map(str::to_string))
        .unwrap_or_else(|| format!("sin constraint: {err}"))
}

async fn raw_insert_debt(
    pool: &PgPool,
    creditor: &str,
    amount: i64,
    issued: NaiveDate,
    due: Option<NaiveDate>,
    key: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO debts (creditor_name, category, nature, concept, original_amount,
                            issued_on, due_on, idempotency_key)
         VALUES ($1, 'maintenance', 'operating', 'c', $2, $3, $4, $5)",
    )
    .bind(creditor)
    .bind(amount)
    .bind(issued)
    .bind(due)
    .bind(key)
    .execute(pool)
    .await
    .map(|_| ())
}

// ============================================================
// 1. RESTRICCIONES (SQL directo)
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn check_constraints_reject_invalid_rows(pool: PgPool) {
    let t = today(&pool).await;

    let e = raw_insert_debt(&pool, "A", 0, t, None, Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(constraint_of(e), "ck_debts_amount_positive");

    let e = raw_insert_debt(&pool, "   ", 10, t, None, Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(constraint_of(e), "ck_debts_creditor_not_blank");

    let e = raw_insert_debt(
        &pool,
        "A",
        10,
        t,
        Some(t - Duration::days(1)),
        Uuid::new_v4(),
    )
    .await
    .unwrap_err();
    assert_eq!(constraint_of(e), "ck_debts_due_after_issue");

    let key = Uuid::new_v4();
    raw_insert_debt(&pool, "A", 10, t, None, key).await.unwrap();
    let e = raw_insert_debt(&pool, "B", 10, t, None, key)
        .await
        .unwrap_err();
    assert_eq!(constraint_of(e), "uq_debts_idempotency_key");

    // Anulación inconsistente: voided_at sin motivo.
    let e = sqlx::query("UPDATE debts SET voided_at = NOW()")
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(constraint_of(e), "ck_debts_void_consistent");
}

// ============================================================
// 2. TRIGGERS (SQL directo)
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn triggers_block_delete_truncate_and_payment_tampering(pool: PgPool) {
    let id = create_with_due(&pool, Some(10), 100_000).await;
    let t = today(&pool).await;
    let payment = repo::register_payment(&pool, id, pay_dto(30_000, t))
        .await
        .unwrap();

    assert!(sqlx::query("DELETE FROM debts")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM debt_payments")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("TRUNCATE debts").execute(&pool).await.is_err());
    assert!(sqlx::query("TRUNCATE debt_payments")
        .execute(&pool)
        .await
        .is_err());

    // Importe de un pago: inmutable. Un UPDATE que no anula tampoco vale.
    assert!(sqlx::query("UPDATE debt_payments SET amount = 1")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("UPDATE debt_payments SET notes = 'x'")
        .execute(&pool)
        .await
        .is_err());
    // Anular cambiando además el importe: rechazado.
    assert!(sqlx::query(
        "UPDATE debt_payments SET amount = 1, voided_at = NOW(), void_reason = 'x'"
    )
    .execute(&pool)
    .await
    .is_err());

    assert_eq!(count(&pool, "debts").await, 1);
    assert_eq!(count(&pool, "debt_payments").await, 1);
    assert_eq!(payment.amount, 30_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn triggers_reject_overpayment_and_voided_debt_payments_even_via_sql(pool: PgPool) {
    let id = create_with_due(&pool, Some(10), 100_000).await;
    let t = today(&pool).await;

    let insert = |amount: i64, paid_on: NaiveDate| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "INSERT INTO debt_payments (debt_id, amount, paid_on, method, idempotency_key)
                 VALUES ($1, $2, $3, 'cash', $4)",
            )
            .bind(id)
            .bind(amount)
            .bind(paid_on)
            .bind(Uuid::new_v4())
            .execute(&pool)
            .await
        }
    };

    assert!(insert(100_001, t).await.is_err(), "sobrepago");
    assert!(
        insert(1, t - Duration::days(31)).await.is_err(),
        "pago anterior a la emisión"
    );
    insert(100_000, t).await.unwrap();
    assert!(insert(1, t).await.is_err(), "ya no hay saldo");

    // Deuda anulada (tras reversar el pago): no admite pagos.
    sqlx::query("UPDATE debt_payments SET voided_at = NOW(), void_reason = 'x'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE debts SET voided_at = NOW(), void_reason = 'x'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(insert(1, t).await.is_err(), "deuda anulada");
}

#[sqlx::test(migrations = "./migrations")]
async fn trigger_blocks_voiding_a_debt_with_live_payments_and_amount_changes(pool: PgPool) {
    let id = create_with_due(&pool, Some(10), 100_000).await;
    let t = today(&pool).await;
    repo::register_payment(&pool, id, pay_dto(10_000, t))
        .await
        .unwrap();

    assert!(
        sqlx::query("UPDATE debts SET voided_at = NOW(), void_reason = 'x'")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("UPDATE debts SET original_amount = 50000")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("UPDATE debts SET issued_on = $1")
        .bind(t + Duration::days(1))
        .execute(&pool)
        .await
        .is_err());
}

// ============================================================
// 3. CREACIÓN
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn create_persists_and_writes_one_audit_event_with_actor(pool: PgPool) {
    let t = today(&pool).await;
    let debt = repo::create_debt(&pool, debt_dto(t, Some(t + Duration::days(20)), 250_000))
        .await
        .unwrap();

    assert_eq!(debt.created_by, "Ana");
    assert_eq!(count(&pool, "debts").await, 1);

    let (module, action, actor, entity_type, entity_id, has_corr): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
    ) = sqlx::query_as(
        "SELECT module::text, action, actor, entity_type, entity_id,
                correlation_id IS NOT NULL FROM audit_events",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((module.as_str(), action.as_str()), ("debts", "create"));
    assert_eq!(actor.as_deref(), Some("Ana"));
    assert_eq!(entity_type.as_deref(), Some("debt"));
    assert_eq!(entity_id, Some(debt.id.to_string()));
    assert!(has_corr);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_is_idempotent_and_detects_key_reuse_with_other_data(pool: PgPool) {
    let t = today(&pool).await;
    let dto = debt_dto(t, None, 100_000);

    let first = repo::create_debt(&pool, dto.clone()).await.unwrap();
    let again = repo::create_debt(&pool, dto.clone()).await.unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(count(&pool, "debts").await, 1);
    assert_eq!(debt_audit_count(&pool).await, 1, "la repetición no audita");

    let mut other = dto.clone();
    other.original_amount = 999;
    assert!(matches!(
        repo::create_debt(&pool, other).await,
        Err(DebtError::IdempotencyConflict)
    ));
    assert_eq!(count(&pool, "debts").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_rejects_invalid_dates_and_amounts_without_side_effects(pool: PgPool) {
    let t = today(&pool).await;

    assert!(matches!(
        repo::create_debt(&pool, debt_dto(t + Duration::days(1), None, 100)).await,
        Err(DebtError::IssuedInFuture)
    ));
    assert!(matches!(
        repo::create_debt(&pool, debt_dto(t, Some(t - Duration::days(1)), 100)).await,
        Err(DebtError::DueBeforeIssue)
    ));
    assert!(matches!(
        repo::create_debt(&pool, debt_dto(t, None, 0)).await,
        Err(DebtError::InvalidAmount)
    ));
    // Emitida hoy y vence hoy: válido.
    repo::create_debt(&pool, debt_dto(t, Some(t), 100))
        .await
        .unwrap();

    assert_eq!(count(&pool, "debts").await, 1);
    assert_eq!(debt_audit_count(&pool).await, 1);
}

// ============================================================
// 4. PAGOS
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn partial_then_full_payment_drive_status_and_balance(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;

    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.balance, o.paid_amount),
        (DebtStatus::Pending, 100_000, 0)
    );

    repo::register_payment(&pool, id, pay_dto(40_000, t))
        .await
        .unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.balance, o.paid_amount),
        (DebtStatus::PartiallyPaid, 60_000, 40_000)
    );
    assert_eq!(o.payment_count, 1);

    repo::register_payment(&pool, id, pay_dto(60_000, t))
        .await
        .unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!((o.status, o.balance), (DebtStatus::Paid, 0));
    assert_eq!(
        o.urgency,
        DebtUrgency::None,
        "una deuda pagada no tiene urgencia"
    );
    assert_eq!(o.last_paid_on, Some(t));

    let payments = repo::list_payments(&pool, id).await.unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(debt_audit_count(&pool).await, 3, "1 deuda + 2 pagos");
}

#[sqlx::test(migrations = "./migrations")]
async fn overpayment_is_rejected_without_row_or_event(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;
    repo::register_payment(&pool, id, pay_dto(70_000, t))
        .await
        .unwrap();
    let events_before = debt_audit_count(&pool).await;

    let err = repo::register_payment(&pool, id, pay_dto(30_001, t))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DebtError::Overpayment {
            balance: 30_000,
            requested: 30_001
        }
    ));

    assert_eq!(count(&pool, "debt_payments").await, 1);
    assert_eq!(debt_audit_count(&pool).await, events_before);

    // Exactamente el saldo sí se acepta.
    repo::register_payment(&pool, id, pay_dto(30_000, t))
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn payment_date_and_amount_rules(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;

    assert!(matches!(
        repo::register_payment(&pool, id, pay_dto(100, t + Duration::days(1))).await,
        Err(DebtError::PaymentInFuture)
    ));
    assert!(matches!(
        repo::register_payment(&pool, id, pay_dto(100, t - Duration::days(31))).await,
        Err(DebtError::PaymentBeforeIssue)
    ));
    assert!(matches!(
        repo::register_payment(&pool, id, pay_dto(0, t)).await,
        Err(DebtError::InvalidAmount)
    ));
    assert!(matches!(
        repo::register_payment(&pool, 9999, pay_dto(100, t)).await,
        Err(DebtError::NotFound)
    ));
    // El día de la emisión y hoy son válidos.
    repo::register_payment(&pool, id, pay_dto(100, t - Duration::days(30)))
        .await
        .unwrap();
    repo::register_payment(&pool, id, pay_dto(100, t))
        .await
        .unwrap();
    assert_eq!(count(&pool, "debt_payments").await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn payment_retry_is_idempotent_even_after_the_debt_is_fully_paid(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;

    let dto = pay_dto(100_000, t);
    let first = repo::register_payment(&pool, id, dto.clone())
        .await
        .unwrap();
    // Reintento tras saldar: no debe fallar por «sobrepago» ni duplicar.
    let again = repo::register_payment(&pool, id, dto.clone())
        .await
        .unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(count(&pool, "debt_payments").await, 1);
    assert_eq!(debt_audit_count(&pool).await, 2);

    let mut different = dto.clone();
    different.amount = 5;
    assert!(matches!(
        repo::register_payment(&pool, id, different).await,
        Err(DebtError::IdempotencyConflict)
    ));

    // Misma clave usada en otra deuda: conflicto.
    let other = create_with_due(&pool, Some(30), 50_000).await;
    assert!(matches!(
        repo::register_payment(&pool, other, dto).await,
        Err(DebtError::IdempotencyConflict)
    ));
    assert_eq!(count(&pool, "debt_payments").await, 1);
}

// ============================================================
// 5. ANULACIÓN DE PAGOS
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn voiding_a_payment_restores_balance_keeps_the_row_and_audits(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;
    let payment = repo::register_payment(&pool, id, pay_dto(100_000, t))
        .await
        .unwrap();
    assert_eq!(
        repo::get_overview(&pool, id).await.unwrap().unwrap().status,
        DebtStatus::Paid
    );

    assert!(matches!(
        repo::void_payment(&pool, payment.id, void_pay_dto("   ")).await,
        Err(DebtError::ReasonRequired)
    ));
    assert!(matches!(
        repo::void_payment(&pool, 9999, void_pay_dto("x")).await,
        Err(DebtError::PaymentNotFound)
    ));

    let voided = repo::void_payment(&pool, payment.id, void_pay_dto("Monto mal digitado"))
        .await
        .unwrap();
    assert!(voided.voided_at.is_some());
    assert_eq!(voided.void_reason.as_deref(), Some("Monto mal digitado"));
    assert_eq!(voided.voided_by.as_deref(), Some("Ana"));

    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.balance, o.paid_amount, o.payment_count),
        (DebtStatus::Pending, 100_000, 0, 0)
    );
    assert_eq!(count(&pool, "debt_payments").await, 1, "el pago permanece");
    assert_eq!(repo::list_payments(&pool, id).await.unwrap().len(), 1);

    assert!(matches!(
        repo::void_payment(&pool, payment.id, void_pay_dto("otra vez")).await,
        Err(DebtError::PaymentAlreadyVoided)
    ));

    let (action, reason): (String, Option<String>) = sqlx::query_as(
        "SELECT action, metadata->>'reason' FROM audit_events
         WHERE entity_type = 'debt_payment' AND action = 'void'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(action, "void");
    assert_eq!(reason.as_deref(), Some("Monto mal digitado"));

    // El saldo liberado se puede volver a pagar (corrección = reverso + pago nuevo).
    repo::register_payment(&pool, id, pay_dto(100_000, t))
        .await
        .unwrap();
    assert_eq!(
        repo::get_overview(&pool, id).await.unwrap().unwrap().status,
        DebtStatus::Paid
    );
}

// ============================================================
// 6. ANULACIÓN DE DEUDAS
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn voiding_a_debt_requires_reversing_payments_first(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;
    let payment = repo::register_payment(&pool, id, pay_dto(10_000, t))
        .await
        .unwrap();

    assert!(matches!(
        repo::void_debt(&pool, id, void_dto("   ")).await,
        Err(DebtError::ReasonRequired)
    ));
    assert!(matches!(
        repo::void_debt(&pool, id, void_dto("Duplicada")).await,
        Err(DebtError::HasLivePayments)
    ));
    assert!(matches!(
        repo::void_debt(&pool, 9999, void_dto("x")).await,
        Err(DebtError::NotFound)
    ));

    repo::void_payment(&pool, payment.id, void_pay_dto("Reverso"))
        .await
        .unwrap();
    let voided = repo::void_debt(&pool, id, void_dto("Duplicada"))
        .await
        .unwrap();
    assert!(voided.voided_at.is_some());
    assert_eq!(voided.void_reason.as_deref(), Some("Duplicada"));

    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.urgency),
        (DebtStatus::Voided, DebtUrgency::None)
    );

    assert!(matches!(
        repo::void_debt(&pool, id, void_dto("otra")).await,
        Err(DebtError::DebtAlreadyVoided)
    ));
    assert!(matches!(
        repo::register_payment(&pool, id, pay_dto(100, t)).await,
        Err(DebtError::DebtVoided)
    ));
    assert_eq!(count(&pool, "debts").await, 1, "la deuda no se borra");
}

// ============================================================
// 7. EDICIÓN
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn update_rules_and_audit_diff(pool: PgPool) {
    let t = today(&pool).await;
    let original = debt_dto(
        t - Duration::days(30),
        Some(t + Duration::days(10)),
        100_000,
    );
    let debt = repo::create_debt(&pool, original.clone()).await.unwrap();
    let events = debt_audit_count(&pool).await;

    // Sin cambios: ni UPDATE ni evento.
    let same = repo::update_debt(&pool, debt.id, update_dto_from(&original))
        .await
        .unwrap();
    assert_eq!(same.updated_at, debt.updated_at);
    assert_eq!(debt_audit_count(&pool).await, events);

    // Cambio real: el evento guarda solo lo modificado, con antes y después.
    let mut edit = update_dto_from(&original);
    edit.concept = "Techo y canaletas".to_string();
    edit.original_amount = 120_000;
    edit.due_on = None;
    let updated = repo::update_debt(&pool, debt.id, edit.clone())
        .await
        .unwrap();
    assert_eq!(updated.original_amount, 120_000);
    assert_eq!(updated.due_on, None);

    let (actor, changes): (Option<String>, serde_json::Value) =
        sqlx::query_as("SELECT actor, changes FROM audit_events WHERE action = 'update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(actor.as_deref(), Some("Luis"));
    let obj = changes.as_object().unwrap();
    assert_eq!(obj.len(), 3, "concept, original_amount y due_on: {obj:?}");
    assert_eq!(obj["original_amount"]["from"], 100_000);
    assert_eq!(obj["original_amount"]["to"], 120_000);
    assert!(obj["due_on"]["to"].is_null());

    // Reglas de fechas e importe.
    let mut bad = edit.clone();
    bad.issued_on = t + Duration::days(1);
    assert!(matches!(
        repo::update_debt(&pool, debt.id, bad).await,
        Err(DebtError::IssuedInFuture)
    ));
    let mut bad = edit.clone();
    bad.due_on = Some(edit.issued_on - Duration::days(1));
    assert!(matches!(
        repo::update_debt(&pool, debt.id, bad).await,
        Err(DebtError::DueBeforeIssue)
    ));
    let mut bad = edit.clone();
    bad.original_amount = 0;
    assert!(matches!(
        repo::update_debt(&pool, debt.id, bad).await,
        Err(DebtError::InvalidAmount)
    ));
    assert!(matches!(
        repo::update_debt(&pool, 9999, edit.clone()).await,
        Err(DebtError::NotFound)
    ));

    // Con un pago vigente: el importe se bloquea y la emisión no pasa del primer pago.
    repo::register_payment(&pool, debt.id, pay_dto(10_000, t - Duration::days(5)))
        .await
        .unwrap();
    let mut locked = edit.clone();
    locked.original_amount = 50_000;
    assert!(matches!(
        repo::update_debt(&pool, debt.id, locked).await,
        Err(DebtError::AmountChangeWithPayments)
    ));
    let mut late_issue = edit.clone();
    late_issue.issued_on = t - Duration::days(4);
    assert!(matches!(
        repo::update_debt(&pool, debt.id, late_issue).await,
        Err(DebtError::IssuedAfterFirstPayment)
    ));
    let mut ok = edit.clone();
    ok.issued_on = t - Duration::days(5); // igual al primer pago: permitido
    ok.notes = Some("Nota".to_string());
    repo::update_debt(&pool, debt.id, ok.clone()).await.unwrap();

    // Anulada: no se edita.
    let other = repo::create_debt(&pool, debt_dto(t, None, 10))
        .await
        .unwrap();
    repo::void_debt(&pool, other.id, void_dto("x"))
        .await
        .unwrap();
    let mut dto = update_dto_from(&debt_dto(t, None, 10));
    dto.concept = "otro".to_string();
    assert!(matches!(
        repo::update_debt(&pool, other.id, dto).await,
        Err(DebtError::DebtVoided)
    ));
}

// ============================================================
// 8. TABLA DE VERDAD: ESTADO Y URGENCIA
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn status_and_urgency_truth_table(pool: PgPool) {
    let t = today(&pool).await;

    // (vencimiento relativo a hoy, urgencia esperada, estado esperado sin pagos)
    let cases: [(Option<i64>, DebtUrgency, DebtStatus); 6] = [
        (Some(8), DebtUrgency::Ok, DebtStatus::Pending),
        (Some(7), DebtUrgency::Alert, DebtStatus::Pending),
        (Some(1), DebtUrgency::Alert, DebtStatus::Pending),
        (Some(0), DebtUrgency::DueToday, DebtStatus::Pending),
        (Some(-1), DebtUrgency::Overdue, DebtStatus::Overdue),
        (None, DebtUrgency::None, DebtStatus::Pending),
    ];
    for (offset, urgency, status) in cases {
        let id = create_with_due(&pool, offset, 100_000).await;
        let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
        assert_eq!((o.urgency, o.status), (urgency, status), "vence {offset:?}");
        assert_eq!(
            o.days_until_due,
            offset.map(|d| d as i32),
            "vence {offset:?}"
        );
        assert_eq!(o.today, t);
    }

    // Pago parcial y todavía no vence: partially_paid, con urgencia propia.
    let id = create_with_due(&pool, Some(5), 100_000).await;
    repo::register_payment(&pool, id, pay_dto(1, t))
        .await
        .unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.urgency),
        (DebtStatus::PartiallyPaid, DebtUrgency::Alert)
    );

    // Vencida con abonos: overdue gana sobre partially_paid.
    let id = create_with_due(&pool, Some(-3), 100_000).await;
    repo::register_payment(&pool, id, pay_dto(1, t))
        .await
        .unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.urgency, o.days_until_due),
        (DebtStatus::Overdue, DebtUrgency::Overdue, Some(-3))
    );

    // Vencida pero saldada: paid, sin urgencia (no figura como vencida).
    repo::register_payment(&pool, id, pay_dto(99_999, t))
        .await
        .unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!((o.status, o.urgency), (DebtStatus::Paid, DebtUrgency::None));

    // Anulada con vencimiento pasado: voided, sin urgencia.
    let id = create_with_due(&pool, Some(-3), 100_000).await;
    repo::void_debt(&pool, id, void_dto("Error")).await.unwrap();
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.status, o.urgency),
        (DebtStatus::Voided, DebtUrgency::None)
    );

    assert!(repo::get_overview(&pool, 9999).await.unwrap().is_none());
}

// ============================================================
// 9. ZONA HORARIA
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn today_and_days_until_due_follow_the_settings_timezone(pool: PgPool) {
    sqlx::query("UPDATE app_settings SET timezone = 'Pacific/Pago_Pago' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let west = today(&pool).await; // UTC-11

    let dto = debt_dto(west, Some(west + Duration::days(1)), 100_000);
    let id = repo::create_debt(&pool, dto).await.unwrap().id;
    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        (o.today, o.days_until_due, o.urgency),
        (west, Some(1), DebtUrgency::Alert)
    );

    sqlx::query("UPDATE app_settings SET timezone = 'Pacific/Kiritimati' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let east = today(&pool).await; // UTC+14: 25 horas por delante
    assert!(east > west, "{east} debería ser posterior a {west}");

    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!(o.today, east);
    assert_eq!(
        o.days_until_due,
        Some(((west + Duration::days(1)) - east).num_days() as i32)
    );
    assert!(o.days_until_due.unwrap() <= 0);
    assert!(matches!(
        o.urgency,
        DebtUrgency::DueToday | DebtUrgency::Overdue
    ));
}

// ============================================================
// 10. ATOMICIDAD
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn failed_audit_insert_rolls_back_every_debt_operation(pool: PgPool) {
    let t = today(&pool).await;

    // Con la auditoría caída no se crea ni la deuda.
    make_audit_fail(&pool).await;
    assert!(repo::create_debt(&pool, debt_dto(t, None, 100_000))
        .await
        .is_err());
    assert_eq!(count(&pool, "debts").await, 0);

    // Con una deuda ya creada, tampoco quedan pagos, ediciones ni anulaciones.
    sqlx::query("DROP TRIGGER trg_fail_audit_insert ON audit_events")
        .execute(&pool)
        .await
        .unwrap();
    let dto = debt_dto(t, None, 100_000);
    let id = repo::create_debt(&pool, dto.clone()).await.unwrap().id;
    let payment = repo::register_payment(&pool, id, pay_dto(10_000, t))
        .await
        .unwrap();
    let events = count(&pool, "audit_events").await;

    sqlx::query(
        "CREATE TRIGGER trg_fail_audit_insert BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION fail_audit_insert()",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(repo::register_payment(&pool, id, pay_dto(10_000, t))
        .await
        .is_err());
    assert_eq!(count(&pool, "debt_payments").await, 1);

    let mut edit = update_dto_from(&dto);
    edit.concept = "Cambio".to_string();
    assert!(repo::update_debt(&pool, id, edit).await.is_err());
    assert_ne!(
        repo::get_overview(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .concept,
        "Cambio"
    );

    assert!(repo::void_payment(&pool, payment.id, void_pay_dto("x"))
        .await
        .is_err());
    assert_eq!(
        repo::get_overview(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .paid_amount,
        10_000,
        "el pago sigue vigente"
    );

    assert_eq!(count(&pool, "audit_events").await, events);
}

// ============================================================
// 11. CONCURRENCIA
// ============================================================

#[sqlx::test(migrations = "./migrations")]
async fn two_simultaneous_payments_cannot_exceed_the_balance(pool: PgPool) {
    let id = create_with_due(&pool, Some(30), 100_000).await;
    let t = today(&pool).await;

    let (a, b) = tokio::join!(
        repo::register_payment(&pool, id, pay_dto(60_000, t)),
        repo::register_payment(&pool, id, pay_dto(60_000, t)),
    );

    let ok = [&a, &b].iter().filter(|r| r.is_ok()).count();
    let overpaid = [&a, &b]
        .iter()
        .filter(|r| matches!(r, Err(DebtError::Overpayment { .. })))
        .count();
    assert_eq!((ok, overpaid), (1, 1), "a = {a:?}, b = {b:?}");

    let o = repo::get_overview(&pool, id).await.unwrap().unwrap();
    assert_eq!((o.paid_amount, o.balance), (60_000, 40_000));
    assert_eq!(count(&pool, "debt_payments").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn payment_racing_against_debt_void_never_leaves_a_voided_debt_with_payments(pool: PgPool) {
    let t = today(&pool).await;

    for _ in 0..8 {
        let id = create_with_due(&pool, Some(30), 100_000).await;
        let (pay, void) = tokio::join!(
            repo::register_payment(&pool, id, pay_dto(40_000, t)),
            repo::void_debt(&pool, id, void_dto("Carrera")),
        );
        assert!(
            !(pay.is_ok() && void.is_ok()),
            "pago y anulación no pueden ganar los dos: {pay:?} / {void:?}"
        );
    }

    let broken: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM debts d
         WHERE d.voided_at IS NOT NULL
           AND EXISTS (SELECT 1 FROM debt_payments p
                       WHERE p.debt_id = d.id AND p.voided_at IS NULL)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(broken, 0);
}

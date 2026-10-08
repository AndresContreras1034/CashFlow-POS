use pos_lib::errors::app_error::AppError;
use pos_lib::modules::audit::{
    dto::{AuditEventFilterDto, NewAuditEvent},
    failure::{finish, FailureContext},
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use pos_lib::modules::cash::{dto::OpenSessionDto, service as cash_service};
use sqlx::PgPool;
use uuid::Uuid;

fn event(module: AuditModule, action: &str, correlation_id: Option<Uuid>) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id,
        category: AuditCategory::Business,
        module,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor: None,
        entity_type: None,
        entity_id: None,
        summary: format!("evento {action}"),
        changes: None,
        metadata: None,
        error_message: None,
    }
}

fn open_dto() -> OpenSessionDto {
    OpenSessionDto {
        opening_amount: 100_000,
        opening_notes: None,
        opened_by: Some("Ana".to_string()),
    }
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

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn open_session_writes_one_audit_event_with_actor(pool: PgPool) {
    cash_service::open_session(&pool, open_dto()).await.unwrap();

    assert_eq!(count(&pool, "cash_sessions").await, 1);
    let (module, action, actor, has_corr): (String, String, Option<String>, bool) = sqlx::query_as(
        "SELECT module::text, action, actor, correlation_id IS NOT NULL FROM audit_events",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((module.as_str(), action.as_str()), ("cash", "open"));
    assert_eq!(actor.as_deref(), Some("Ana"));
    assert!(has_corr);
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_audit_insert_rolls_back_the_business_operation(pool: PgPool) {
    make_audit_fail(&pool).await;

    let result = cash_service::open_session(&pool, open_dto()).await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "cash_sessions").await,
        0,
        "el turno no debe quedar"
    );
    assert_eq!(count(&pool, "audit_events").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn technical_failure_is_recorded_and_client_error_is_unchanged(pool: PgPool) {
    let ctx = FailureContext {
        module: AuditModule::Cash,
        action: "open",
        entity_type: Some("cash_session"),
        entity_id: None,
        actor: Some("Ana".to_string()),
    };
    let result: Result<(), String> = finish(&pool, ctx, Err(AppError::internal("boom"))).await;
    assert_eq!(result.unwrap_err(), "Error interno: boom");

    let (category, outcome, message): (String, String, Option<String>) =
        sqlx::query_as("SELECT category::text, outcome::text, error_message FROM audit_events")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((category.as_str(), outcome.as_str()), ("error", "failure"));
    assert_eq!(message.as_deref(), Some("boom"));
}

#[sqlx::test(migrations = "./migrations")]
async fn business_rejections_leave_no_audit_event(pool: PgPool) {
    let ctx = FailureContext {
        module: AuditModule::Cash,
        action: "open",
        entity_type: None,
        entity_id: None,
        actor: None,
    };
    let result: Result<(), String> =
        finish(&pool, ctx, Err(AppError::validation("monto inválido"))).await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "audit_events").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_events_filters_paginates_and_orders(pool: PgPool) {
    let correlation_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.unwrap();
    audit_repo::insert_event(
        &mut *conn,
        event(AuditModule::Cash, "open", Some(correlation_id)),
    )
    .await
    .unwrap();
    audit_repo::insert_event(&mut *conn, event(AuditModule::Cash, "close", None))
        .await
        .unwrap();
    audit_repo::insert_event(&mut *conn, event(AuditModule::Sales, "create", None))
        .await
        .unwrap();
    drop(conn);

    let by_module = AuditEventFilterDto {
        module: Some(AuditModule::Cash),
        page_size: Some(1),
        ..Default::default()
    };
    let (rows, total) = audit_repo::list_events(&pool, &by_module).await.unwrap();
    assert_eq!(total, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].action, "close", "más reciente primero");

    let by_correlation = AuditEventFilterDto {
        correlation_id: Some(correlation_id),
        ..Default::default()
    };
    let (rows, total) = audit_repo::list_events(&pool, &by_correlation)
        .await
        .unwrap();
    assert_eq!((total, rows[0].action.as_str()), (1, "open"));

    let all = audit_repo::list_by_correlation(&pool, correlation_id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn audit_table_is_append_only(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    audit_repo::insert_event(&mut *conn, event(AuditModule::Cash, "open", None))
        .await
        .unwrap();
    drop(conn);

    assert!(sqlx::query("UPDATE audit_events SET summary = 'x'")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM audit_events")
        .execute(&pool)
        .await
        .is_err());
    assert_eq!(count(&pool, "audit_events").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn backup_encryption_is_enabled_by_default_for_existing_and_new_rows(pool: sqlx::PgPool) {
    let (enabled, default_value): (bool, Option<String>) = sqlx::query_as(
        "SELECT encrypt_enabled,
                (SELECT column_default
                 FROM information_schema.columns
                 WHERE table_schema = 'public'
                   AND table_name = 'backup_settings'
                   AND column_name = 'encrypt_enabled')
         FROM backup_settings
         WHERE id = 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(enabled);
    assert_eq!(default_value.as_deref(), Some("true"));
}

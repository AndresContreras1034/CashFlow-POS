use chrono::{Duration, Utc};
use pos_lib::modules::backup::{repository, retention};
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn rotation_ignores_a_manipulated_file_path(pool: sqlx::PgPool) {
    let root = std::env::temp_dir().join(format!("cashflow_rotation_{}", Uuid::new_v4()));
    let backup_dir = root.join("backups");
    let outside_dir = root.join("outside");
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();

    sqlx::query(
        "UPDATE backup_settings
         SET keep_daily = 1, keep_weekly = 0, keep_monthly = 0
         WHERE id = 1",
    )
    .execute(&pool)
    .await
    .unwrap();

    let now = Utc::now();
    let old_name = "cashflow_2026-10-15_100000.dump";
    let latest_name = "cashflow_2026-10-15_110000.dump";
    let old_managed_file = backup_dir.join(old_name);
    let old_external_file = outside_dir.join(old_name);
    let latest_file = backup_dir.join(latest_name);
    std::fs::write(&old_managed_file, b"managed").unwrap();
    std::fs::write(&old_external_file, b"outside").unwrap();
    std::fs::write(&latest_file, b"latest").unwrap();

    for (started_at, name, file_path) in [
        (
            now - Duration::hours(2),
            old_name,
            old_external_file.as_path(),
        ),
        (now - Duration::hours(1), latest_name, latest_file.as_path()),
    ] {
        sqlx::query(
            "INSERT INTO backup_history
                (kind, status, started_at, duration_ms, file_name, file_path,
                 size_bytes, sha256, pg_server_version, last_migration, created_by)
             VALUES ('manual', 'success', $1, 0, $2, $3, 1, repeat('a', 64),
                     'PostgreSQL 18', 34, 'test')",
        )
        .bind(started_at)
        .bind(name)
        .bind(file_path.to_string_lossy().into_owned())
        .execute(&pool)
        .await
        .unwrap();
    }

    let settings = repository::get_settings(&pool).await.unwrap().unwrap();
    let removed = retention::apply(&pool, &settings, &backup_dir)
        .await
        .unwrap();

    assert_eq!(removed, 1);
    assert!(!old_managed_file.exists());
    assert!(old_external_file.exists());
    assert!(latest_file.exists());

    let _ = std::fs::remove_dir_all(&root);
}

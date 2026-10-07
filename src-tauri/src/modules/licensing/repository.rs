use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct LicenseState {
    pub installation_id: String,
    pub license_file: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

pub async fn get_state(pool: &PgPool) -> Result<LicenseState, sqlx::Error> {
    let new_installation_id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO license_state (id, installation_id)
         VALUES (1, $1)
         ON CONFLICT (id) DO NOTHING",
        new_installation_id
    )
    .execute(pool)
    .await?;

    let row = sqlx::query!(
        "SELECT installation_id, license_file, last_seen_at
         FROM license_state
         WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;

    Ok(LicenseState {
        installation_id: row.installation_id,
        license_file: row.license_file,
        last_seen_at: row.last_seen_at,
    })
}

pub async fn save_license(pool: &PgPool, license_file: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE license_state SET license_file = $1 WHERE id = 1",
        license_file
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_last_seen(pool: &PgPool, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE license_state
         SET last_seen_at = $1
         WHERE id = 1",
        now
    )
    .execute(pool)
    .await?;
    Ok(())
}

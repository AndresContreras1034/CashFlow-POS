use chrono::{DateTime, Duration, Utc};
use rsa::{
    pkcs1v15::{Signature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use sha2::Sha256;
use sqlx::PgPool;

use crate::{errors::app_error::AppError, modules::licensing::models::*};

use super::{public_key, repository};

const CLOCK_SKEW: Duration = Duration::minutes(5);
const MAX_LICENSE_FILE_BYTES: usize = 16 * 1024;

fn decode_hex(hex: &str) -> Result<Vec<u8>, AppError> {
    if hex.is_empty() || hex.len() % 2 != 0 {
        return Err(AppError::validation("La firma de la licencia no es válida"));
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| AppError::validation("La firma de la licencia no es válida"))?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| AppError::validation("La firma de la licencia no es válida"))?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

fn verify_signed_license(file: &str) -> Result<(SignedLicense, LicensePayload), AppError> {
    let signed: SignedLicense = serde_json::from_str(file)
        .map_err(|_| AppError::validation("El archivo de licencia tiene un formato inválido"))?;
    let signature_bytes = decode_hex(&signed.signature)?;
    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| AppError::validation("La firma de la licencia no es válida"))?;
    let public_key = RsaPublicKey::from_public_key_pem(public_key::LICENSE_PUBLIC_KEY)
        .map_err(|_| AppError::internal("La clave pública de licencias no está configurada"))?;
    VerifyingKey::<Sha256>::new(public_key)
        .verify(signed.payload.as_bytes(), &signature)
        .map_err(|_| AppError::validation("La firma de la licencia no coincide"))?;

    let payload: LicensePayload = serde_json::from_str(&signed.payload)
        .map_err(|_| AppError::validation("Los datos de la licencia no son válidos"))?;
    Ok((signed, payload))
}

fn status_for(
    payload: &LicensePayload,
    installation_id: &str,
    last_seen_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> LicenseStatusDto {
    let invalid = |message: &str| LicenseStatusDto {
        installation_id: installation_id.to_string(),
        status: LicenseStatusKind::Invalid,
        license_id: Some(payload.license_id.clone()),
        licensee: Some(payload.licensee.clone()),
        kind: Some(payload.kind),
        expires_at: payload.expires_at.clone(),
        source_code_access: payload.source_code_access,
        message: message.to_string(),
    };

    if payload.schema_version != 1
        || payload.license_id.trim().is_empty()
        || payload.licensee.trim().is_empty()
        || !payload.source_code_access
    {
        return invalid("Los datos de la licencia no cumplen los requisitos.");
    }
    if payload.installation_id != installation_id {
        return invalid("Esta licencia pertenece a otra instalación.");
    }

    let issued_at = match DateTime::parse_from_rfc3339(&payload.issued_at) {
        Ok(value) => value.with_timezone(&Utc),
        Err(_) => return invalid("La fecha de emisión de la licencia no es válida."),
    };
    if issued_at > now + CLOCK_SKEW {
        return invalid("La fecha de emisión está en el futuro.");
    }

    let expires_at = match (&payload.kind, &payload.expires_at) {
        (LicenseKind::Purchase, None) => None,
        (LicenseKind::Rental, Some(value)) => match DateTime::parse_from_rfc3339(value) {
            Ok(value) => Some(value.with_timezone(&Utc)),
            Err(_) => return invalid("La fecha de vencimiento no es válida."),
        },
        _ => return invalid("La modalidad y el vencimiento de la licencia no coinciden."),
    };

    if let Some(last_seen_at) = last_seen_at {
        if now + CLOCK_SKEW < last_seen_at {
            return invalid(
                "Se detectó que el reloj del equipo retrocedió. Corrige la fecha y hora.",
            );
        }
    }

    if let Some(expires_at) = expires_at {
        if expires_at <= now {
            return LicenseStatusDto {
                installation_id: installation_id.to_string(),
                status: LicenseStatusKind::Expired,
                license_id: Some(payload.license_id.clone()),
                licensee: Some(payload.licensee.clone()),
                kind: Some(payload.kind),
                expires_at: payload.expires_at.clone(),
                source_code_access: payload.source_code_access,
                message: "La licencia de alquiler venció. Importa una licencia renovada."
                    .to_string(),
            };
        }
    }

    LicenseStatusDto {
        installation_id: installation_id.to_string(),
        status: LicenseStatusKind::Active,
        license_id: Some(payload.license_id.clone()),
        licensee: Some(payload.licensee.clone()),
        kind: Some(payload.kind),
        expires_at: payload.expires_at.clone(),
        source_code_access: payload.source_code_access,
        message: "Licencia activa.".to_string(),
    }
}

pub async fn get_status(pool: &PgPool) -> Result<LicenseStatusDto, AppError> {
    let state = repository::get_state(pool).await.map_err(AppError::from)?;
    let now = Utc::now();

    let Some(file) = state.license_file else {
        repository::update_last_seen(pool, now)
            .await
            .map_err(AppError::from)?;
        return Ok(LicenseStatusDto {
            installation_id: state.installation_id,
            status: LicenseStatusKind::Unlicensed,
            license_id: None,
            licensee: None,
            kind: None,
            expires_at: None,
            source_code_access: false,
            message: "Esta instalación aún no tiene una licencia.".to_string(),
        });
    };

    let status = match verify_signed_license(&file) {
        Ok((_, payload)) => status_for(&payload, &state.installation_id, state.last_seen_at, now),
        Err(_) => LicenseStatusDto {
            installation_id: state.installation_id.clone(),
            status: LicenseStatusKind::Invalid,
            license_id: None,
            licensee: None,
            kind: None,
            expires_at: None,
            source_code_access: false,
            message: "La licencia guardada no es válida. Importa una licencia válida.".to_string(),
        },
    };

    if !matches!(&status.status, LicenseStatusKind::Invalid) {
        repository::update_last_seen(pool, now)
            .await
            .map_err(AppError::from)?;
    }
    Ok(status)
}

pub async fn activate(pool: &PgPool, path: &str) -> Result<(), AppError> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|error| AppError::validation(&format!("No se pudo leer el archivo: {error}")))?;
    if metadata.len() > MAX_LICENSE_FILE_BYTES as u64 {
        return Err(AppError::validation(
            "El archivo de licencia es demasiado grande",
        ));
    }
    let file = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| AppError::validation(&format!("No se pudo leer el archivo: {error}")))?;
    let (signed, payload) = verify_signed_license(&file)?;
    let state = repository::get_state(pool).await.map_err(AppError::from)?;
    let now = Utc::now();
    let status = status_for(&payload, &state.installation_id, state.last_seen_at, now);
    match status.status {
        LicenseStatusKind::Active => {}
        LicenseStatusKind::Expired => {
            return Err(AppError::validation(&status.message));
        }
        _ => return Err(AppError::validation(&status.message)),
    }

    let stored = serde_json::to_string(&signed)
        .map_err(|error| AppError::internal(&format!("No se pudo guardar la licencia: {error}")))?;
    repository::save_license(pool, &stored)
        .await
        .map_err(AppError::from)?;
    repository::update_last_seen(pool, now)
        .await
        .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(kind: LicenseKind, expires_at: Option<String>) -> LicensePayload {
        LicensePayload {
            schema_version: 1,
            license_id: "license-test".into(),
            installation_id: "installation-test".into(),
            licensee: "AFCM".into(),
            kind,
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at,
            source_code_access: true,
        }
    }

    #[test]
    fn perpetual_purchase_is_active() {
        let now = DateTime::parse_from_rfc3339("2026-10-06T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let result = status_for(
            &payload(LicenseKind::Purchase, None),
            "installation-test",
            None,
            now,
        );
        assert_eq!(result.status, LicenseStatusKind::Active);
    }

    #[test]
    fn expired_rental_is_blocked() {
        let now = DateTime::parse_from_rfc3339("2026-10-06T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let result = status_for(
            &payload(LicenseKind::Rental, Some("2026-10-05T23:59:59Z".into())),
            "installation-test",
            None,
            now,
        );
        assert_eq!(result.status, LicenseStatusKind::Expired);
    }

    #[test]
    fn license_is_bound_to_its_installation() {
        let now = Utc::now();
        let result = status_for(
            &payload(LicenseKind::Purchase, None),
            "another-installation",
            None,
            now,
        );
        assert_eq!(result.status, LicenseStatusKind::Invalid);
    }

    #[test]
    fn clock_rollback_invalidates_license_until_clock_is_corrected() {
        let now = DateTime::parse_from_rfc3339("2026-10-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let last_seen = DateTime::parse_from_rfc3339("2026-10-06T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let result = status_for(
            &payload(LicenseKind::Purchase, None),
            "installation-test",
            Some(last_seen),
            now,
        );
        assert_eq!(result.status, LicenseStatusKind::Invalid);
    }
}

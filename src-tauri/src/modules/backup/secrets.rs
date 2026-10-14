//! Contraseñas en el Administrador de credenciales de Windows.
//! Nunca van a la base de datos: el backup las incluiría.

use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "CashFlow-POS";

#[derive(Debug, Clone, Copy)]
pub enum Secret {
    SmtpPassword,
    EncryptionPassword,
}

impl Secret {
    fn account(self) -> &'static str {
        match self {
            Self::SmtpPassword => "backup-smtp-password",
            Self::EncryptionPassword => "backup-encryption-password",
        }
    }
}

fn entry(secret: Secret) -> Result<Entry, String> {
    Entry::new(SERVICE, secret.account())
        .map_err(|e| format!("No se pudo acceder al almacén de credenciales: {e}"))
}

pub fn get(secret: Secret) -> Result<Option<String>, String> {
    match entry(secret)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(format!("No se pudo leer la credencial guardada: {e}")),
    }
}

pub fn exists(secret: Secret) -> Result<bool, String> {
    get(secret).map(|value| value.is_some())
}

pub fn set(secret: Secret, value: &str) -> Result<(), String> {
    entry(secret)?
        .set_password(value)
        .map_err(|e| format!("No se pudo guardar la credencial: {e}"))
}

pub fn delete(secret: Secret) -> Result<(), String> {
    match entry(secret)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(format!("No se pudo borrar la credencial: {e}")),
    }
}

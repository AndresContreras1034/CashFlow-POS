use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "CashFlow-POS";
const ACCOUNT: &str = "deepseek_api_key";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|e| format!("No se pudo acceder al almacén de credenciales: {e}"))
}

pub fn get_key() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("No se pudo leer la credencial guardada: {error}")),
    }
}

pub fn has_key() -> Result<bool, String> {
    get_key().map(|value| value.is_some())
}

pub fn set_key(value: &str) -> Result<(), String> {
    entry()?
        .set_password(value)
        .map_err(|error| format!("No se pudo guardar la credencial: {error}"))
}

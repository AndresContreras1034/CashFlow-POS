use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    Conflict(String),

    #[error("Error interno de base de datos")]
    Database(#[source] sqlx::Error),

    #[error("Error interno: {0}")]
    Internal(String),
}

impl AppError {
    pub fn not_found(msg: &str) -> Self {
        Self::NotFound(msg.to_string())
    }

    pub fn validation(msg: &str) -> Self {
        Self::Validation(msg.to_string())
    }

    pub fn conflict(msg: &str) -> Self {
        Self::Conflict(msg.to_string())
    }

    pub fn internal(msg: &str) -> Self {
        Self::Internal(msg.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .is_some_and(|database_error| database_error.is_unique_violation())
        {
            tracing::warn!(error = %error, "Database unique constraint conflict");
            Self::conflict("Ya existe un registro con esos datos")
        } else {
            tracing::error!(error = %error, "Database operation failed");
            Self::Database(error)
        }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_error_serialization_does_not_expose_details() {
        let error = AppError::Database(sqlx::Error::Protocol(
            "sensitive statement details".to_string(),
        ));
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            "\"Error interno de base de datos\""
        );
    }
}

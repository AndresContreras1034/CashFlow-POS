use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Validation(String),

    #[error("Error de base de datos: {0}")]
    Database(#[from] sqlx::Error),

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

    pub fn internal(msg: &str) -> Self {
        Self::Internal(msg.to_string())
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

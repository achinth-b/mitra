use crate::database::DatabaseError;
use sqlx::Error as SqlxError;
use thiserror::Error;

/// Application-level error types
#[derive(Error, Debug)]
pub enum AppError {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// SQLx database errors
    #[error("SQL error: {0}")]
    Sqlx(#[from] SqlxError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found errors
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Unauthorized access errors
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Business logic errors
    #[error("Business logic error: {0}")]
    BusinessLogic(String),

    /// External service errors
    #[error("External service error: {0}")]
    ExternalService(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// UUID parsing errors
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    /// Decimal parsing errors
    #[error("Invalid decimal: {0}")]
    InvalidDecimal(String),

    /// Generic error with message
    #[error("{0}")]
    Message(String),
}

/// Result type alias for application errors
pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    /// Check if error is a database connection error
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            AppError::Database(DatabaseError::PoolCreation(_))
                | AppError::Database(DatabaseError::ConnectionTimeout)
        )
    }

    /// Check if error is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, AppError::NotFound(_))
    }

    /// Get HTTP status code for the error
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::NotFound(_) => 404,
            AppError::Unauthorized(_) => 401,
            AppError::Validation(_) => 400,
            AppError::Config(_) => 500,
            AppError::Database(_) | AppError::Sqlx(_) => 500,
            AppError::ExternalService(_) => 502,
            _ => 500,
        }
    }
}

/// Repository-specific error types
#[derive(Error, Debug)]
pub enum RepositoryError {
    /// Database query error
    #[error("Query error: {0}")]
    Query(SqlxError),

    /// Record not found
    #[error("Record not found")]
    NotFound(String),

    /// Duplicate record
    #[error("Duplicate record: {0}")]
    Duplicate(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Business rule violation (e.g., insufficient balance)
    #[error("Business rule violation: {0}")]
    BusinessRule(String),
}

impl From<RepositoryError> for AppError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotFound(msg) => AppError::NotFound(msg),
            RepositoryError::Query(e) => AppError::Sqlx(e),
            RepositoryError::Duplicate(msg) => AppError::BusinessLogic(format!("Duplicate: {}", msg)),
            RepositoryError::ConstraintViolation(msg) => AppError::Validation(msg),
            RepositoryError::InvalidInput(msg) => AppError::Validation(msg),
            RepositoryError::BusinessRule(msg) => AppError::BusinessLogic(msg),
        }
    }
}

impl From<SqlxError> for RepositoryError {
    fn from(err: SqlxError) -> Self {
        match &err {
            SqlxError::RowNotFound => RepositoryError::NotFound("Record not found".to_string()),
            SqlxError::Database(db_err) => {
                // Check for common PostgreSQL error codes
                let code = db_err.code().map(|c| c.to_string());
                if code.as_deref() == Some("23505") {
                    // Unique violation
                    RepositoryError::Duplicate(db_err.message().to_string())
                } else if code.as_deref() == Some("23503") {
                    // Foreign key violation
                    RepositoryError::ConstraintViolation(db_err.message().to_string())
                } else if code.as_deref() == Some("23514") {
                    // Check constraint violation
                    RepositoryError::ConstraintViolation(db_err.message().to_string())
                } else {
                    RepositoryError::Query(err)
                }
            }
            _ => RepositoryError::Query(err),
        }
    }
}

/// Convenience function to convert Option<T> to Result<T, AppError>
pub fn option_to_result<T>(opt: Option<T>, error_msg: &str) -> AppResult<T> {
    opt.ok_or_else(|| AppError::NotFound(error_msg.to_string()))
}

/// Convenience function to convert Result<T, E> to AppResult<T>
pub fn map_to_app_error<T, E: std::error::Error>(result: Result<T, E>, context: &str) -> AppResult<T> {
    result.map_err(|e| AppError::Message(format!("{}: {}", context, e)))
}


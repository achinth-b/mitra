use crate::config::DatabaseConfig;
use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;

/// Errors that can occur when working with the database
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to create database pool: {0}")]
    PoolCreation(sqlx::Error),
    
    #[error("Database query error: {0}")]
    QueryError(sqlx::Error),
    
    #[error("Database connection timeout")]
    ConnectionTimeout,
    
    #[error("Database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        DatabaseError::QueryError(err)
    }
}

/// Database wrapper that holds the connection pool
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new Database instance with the given pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get ownership of the pool (useful for passing to repositories)
    pub fn into_pool(self) -> PgPool {
        self.pool
    }
}

/// Create a PostgreSQL connection pool with optimized settings
///
/// # Arguments
/// * `config` - Database configuration
///
/// # Returns
/// * `Ok(PgPool)` - Successfully created connection pool
/// * `Err(DatabaseError)` - Error creating the pool
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool, DatabaseError> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout())
        .idle_timeout(config.idle_timeout())
        .max_lifetime(config.max_lifetime())
        .test_before_acquire(config.test_before_acquire)
        .connect(&config.url)
        .await
        .map_err(DatabaseError::PoolCreation)?;

    // Test the connection
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(DatabaseError::PoolCreation)?;

    Ok(pool)
}

/// Create a PostgreSQL connection pool from a URL string (legacy method)
/// Prefer using `create_pool` with `DatabaseConfig` instead
pub async fn create_pool_from_url(_database_url: &str) -> Result<PgPool, DatabaseError> {
    let config = DatabaseConfig::from_env()
        .map_err(|e| DatabaseError::Config(e))?;
    create_pool(&config).await
}

/// Run database migrations
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `migrations_path` - Path to migrations directory (default: "./migrations")
///
/// # Returns
/// * `Ok(())` - Migrations completed successfully
/// * `Err(DatabaseError)` - Migration error
pub async fn run_migrations(
    pool: &PgPool,
    migrations_path: Option<&str>,
) -> Result<(), DatabaseError> {
    let path = migrations_path.unwrap_or("./migrations");
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new(path))
        .await
        .map_err(DatabaseError::Migration)?;
    
    migrator.run(pool).await.map_err(DatabaseError::Migration)?;
    
    Ok(())
}
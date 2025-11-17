use std::env;
use std::time::Duration;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub test_before_acquire: bool,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub log_level: String,
    pub grpc_port: u16,
    pub http_port: Option<u16>,
    pub environment: String,
}

impl DatabaseConfig {
    /// Create database config from environment variables
    pub fn from_env() -> Result<Self, String> {
        let url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable is required")?;

        let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(10);

        let acquire_timeout_secs = env::var("DATABASE_ACQUIRE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

        let idle_timeout_secs = env::var("DATABASE_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(600); // 10 minutes

        let max_lifetime_secs = env::var("DATABASE_MAX_LIFETIME_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1800); // 30 minutes

        let test_before_acquire = env::var("DATABASE_TEST_BEFORE_ACQUIRE")
            .ok()
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(true);

        // Validate configuration
        if max_connections == 0 {
            return Err("DATABASE_MAX_CONNECTIONS must be greater than 0".to_string());
        }

        if acquire_timeout_secs == 0 {
            return Err("DATABASE_ACQUIRE_TIMEOUT_SECS must be greater than 0".to_string());
        }

        Ok(Self {
            url,
            max_connections,
            acquire_timeout_secs,
            idle_timeout_secs,
            max_lifetime_secs,
            test_before_acquire,
        })
    }

    /// Get acquire timeout as Duration
    pub fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_secs)
    }

    /// Get idle timeout as Duration
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout_secs)
    }

    /// Get max lifetime as Duration
    pub fn max_lifetime(&self) -> Duration {
        Duration::from_secs(self.max_lifetime_secs)
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/mitra".to_string(),
            max_connections: 10,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            test_before_acquire: true,
        }
    }
}

impl AppConfig {
    /// Create application config from environment variables
    pub fn from_env() -> Result<Self, String> {
        let database = DatabaseConfig::from_env()?;

        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        let grpc_port = env::var("GRPC_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(50051);

        let http_port = env::var("HTTP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok());

        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string());

        // Validate log level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&log_level.to_lowercase().as_str()) {
            return Err(format!(
                "Invalid LOG_LEVEL: {}. Must be one of: {:?}",
                log_level, valid_log_levels
            ));
        }

        // Validate environment
        let valid_environments = ["development", "staging", "production"];
        if !valid_environments.contains(&environment.to_lowercase().as_str()) {
            return Err(format!(
                "Invalid ENVIRONMENT: {}. Must be one of: {:?}",
                environment, valid_environments
            ));
        }

        Ok(Self {
            database,
            log_level: log_level.to_lowercase(),
            grpc_port,
            http_port,
            environment: environment.to_lowercase(),
        })
    }

    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Check if running in development
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Get database URL (convenience method)
    pub fn database_url(&self) -> &str {
        &self.database.url
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            log_level: "info".to_string(),
            grpc_port: 50051,
            http_port: None,
            environment: "development".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.acquire_timeout_secs, 30);
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.grpc_port, 50051);
        assert!(config.is_development());
        assert!(!config.is_production());
    }
}


use crate::models::User;
use sqlx::{PgPool, Result as SqlxResult};
use uuid::Uuid;

/// Repository for user data access
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Create a new UserRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new user
    pub async fn create(&self, wallet_address: &str) -> SqlxResult<User> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (wallet_address)
            VALUES ($1)
            RETURNING id, wallet_address, created_at
            "#,
            wallet_address
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find a user by UUID
    pub async fn find_by_id(&self, id: Uuid) -> SqlxResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, wallet_address, created_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find a user by wallet address
    pub async fn find_by_wallet(&self, wallet_address: &str) -> SqlxResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, wallet_address, created_at
            FROM users
            WHERE wallet_address = $1
            "#,
            wallet_address
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find or create a user by wallet address (upsert pattern)
    /// Returns the user whether it was created or already existed
    pub async fn find_or_create_by_wallet(&self, wallet_address: &str) -> SqlxResult<User> {
        // Try to find existing user first
        if let Some(user) = self.find_by_wallet(wallet_address).await? {
            return Ok(user);
        }

        // Create new user if not found
        self.create(wallet_address).await
    }
}


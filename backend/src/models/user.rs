use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User model representing a user account indexed by Solana wallet address
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub wallet_address: String,
    pub created_at: NaiveDateTime,
}

impl User {
    /// Create a new User (typically used for creating from API input)
    pub fn new(wallet_address: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            wallet_address,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}
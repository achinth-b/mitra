use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Friend Group model representing a group of users who can create and bet on events
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FriendGroup {
    pub id: Uuid,
    pub solana_pubkey: String,
    pub name: String,
    pub admin_wallet: String,
    pub created_at: NaiveDateTime,
}

impl FriendGroup {
    /// Create a new FriendGroup (typically used for creating from API input)
    pub fn new(
        solana_pubkey: String,
        name: String,
        admin_wallet: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            solana_pubkey,
            name,
            admin_wallet,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}
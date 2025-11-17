use crate::models::FriendGroup;
use sqlx::{PgPool, Result as SqlxResult};
use uuid::Uuid;

/// Repository for friend group data access
pub struct FriendGroupRepository {
    pool: PgPool,
}

impl FriendGroupRepository {
    /// Create a new FriendGroupRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new friend group
    pub async fn create(
        &self,
        solana_pubkey: &str,
        name: &str,
        admin_wallet: &str,
    ) -> SqlxResult<FriendGroup> {
        sqlx::query_as!(
            FriendGroup,
            r#"
            INSERT INTO friend_groups (solana_pubkey, name, admin_wallet)
            VALUES ($1, $2, $3)
            RETURNING id, solana_pubkey, name, admin_wallet, created_at
            "#,
            solana_pubkey,
            name,
            admin_wallet
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find a friend group by UUID
    pub async fn find_by_id(&self, id: Uuid) -> SqlxResult<Option<FriendGroup>> {
        sqlx::query_as!(
            FriendGroup,
            r#"
            SELECT id, solana_pubkey, name, admin_wallet, created_at
            FROM friend_groups
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find a friend group by Solana pubkey
    pub async fn find_by_solana_pubkey(&self, pubkey: &str) -> SqlxResult<Option<FriendGroup>> {
        sqlx::query_as!(
            FriendGroup,
            r#"
            SELECT id, solana_pubkey, name, admin_wallet, created_at
            FROM friend_groups
            WHERE solana_pubkey = $1
            "#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Update friend group name
    pub async fn update_name(&self, id: Uuid, name: &str) -> SqlxResult<FriendGroup> {
        sqlx::query_as!(
            FriendGroup,
            r#"
            UPDATE friend_groups
            SET name = $2
            WHERE id = $1
            RETURNING id, solana_pubkey, name, admin_wallet, created_at
            "#,
            id,
            name
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a friend group (cascades to members and events)
    pub async fn delete(&self, id: Uuid) -> SqlxResult<bool> {
        let rows_affected = sqlx::query!(
            r#"
            DELETE FROM friend_groups
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Find all friend groups for an admin wallet
    pub async fn find_by_admin_wallet(&self, admin_wallet: &str) -> SqlxResult<Vec<FriendGroup>> {
        sqlx::query_as!(
            FriendGroup,
            r#"
            SELECT id, solana_pubkey, name, admin_wallet, created_at
            FROM friend_groups
            WHERE admin_wallet = $1
            ORDER BY created_at DESC
            "#,
            admin_wallet
        )
        .fetch_all(&self.pool)
        .await
    }
}


use crate::models::Bet;
use rust_decimal::Decimal;
use sqlx::{PgPool, Result as SqlxResult};
use uuid::Uuid;

/// Repository for bet data access
pub struct BetRepository {
    pool: PgPool,
}

impl BetRepository {
    /// Create a new BetRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new bet
    pub async fn create(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        outcome: &str,
        shares: Decimal,
        price: Decimal,
        amount_usdc: Decimal,
    ) -> SqlxResult<Bet> {
        sqlx::query_as!(
            Bet,
            r#"
            INSERT INTO bets (event_id, user_id, outcome, shares, price, amount_usdc)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            "#,
            event_id,
            user_id,
            outcome,
            shares,
            price,
            amount_usdc
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find a bet by UUID
    pub async fn find_by_id(&self, id: Uuid) -> SqlxResult<Option<Bet>> {
        sqlx::query_as!(
            Bet,
            r#"
            SELECT 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            FROM bets
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find all bets for an event
    pub async fn find_by_event(&self, event_id: Uuid) -> SqlxResult<Vec<Bet>> {
        sqlx::query_as!(
            Bet,
            r#"
            SELECT 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            FROM bets
            WHERE event_id = $1
            ORDER BY timestamp DESC
            "#,
            event_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find all bets for a user
    pub async fn find_by_user(&self, user_id: Uuid) -> SqlxResult<Vec<Bet>> {
        sqlx::query_as!(
            Bet,
            r#"
            SELECT 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            FROM bets
            WHERE user_id = $1
            ORDER BY timestamp DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find bets for a user in a specific event
    pub async fn find_by_user_and_event(
        &self,
        user_id: Uuid,
        event_id: Uuid,
    ) -> SqlxResult<Vec<Bet>> {
        sqlx::query_as!(
            Bet,
            r#"
            SELECT 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            FROM bets
            WHERE user_id = $1 AND event_id = $2
            ORDER BY timestamp DESC
            "#,
            user_id,
            event_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find pending bets (uncommitted - for Phase 7)
    /// Note: This will work once committed_slot column is added
    pub async fn find_pending_bets(&self) -> SqlxResult<Vec<Bet>> {
        // For MVP, all bets are considered "pending" since committed_slot doesn't exist yet
        // This query will need to be updated in Phase 7 to filter by committed_slot IS NULL
        sqlx::query_as!(
            Bet,
            r#"
            SELECT 
                id, 
                event_id, 
                user_id, 
                outcome, 
                shares, 
                price, 
                amount_usdc, 
                timestamp
            FROM bets
            ORDER BY timestamp DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Mark a bet as committed (for Phase 7)
    /// Note: This will be implemented when committed_slot and merkle_proof columns are added
    #[allow(dead_code)]
    pub async fn mark_committed(
        &self,
        _id: Uuid,
        _committed_slot: i64,
        _merkle_proof: &serde_json::Value,
    ) -> SqlxResult<Bet> {
        // This will be uncommented and implemented in Phase 7
        // sqlx::query_as!(
        //     Bet,
        //     r#"
        //     UPDATE bets
        //     SET committed_slot = $2, merkle_proof = $3
        //     WHERE id = $1
        //     RETURNING ...
        //     "#,
        //     id,
        //     committed_slot,
        //     merkle_proof
        // )
        // .fetch_one(&self.pool)
        // .await
        todo!("Implement in Phase 7 when merkle fields are added")
    }

    /// Get total volume (sum of amount_usdc) for an event
    pub async fn get_total_volume_for_event(&self, event_id: Uuid) -> SqlxResult<Option<Decimal>> {
        let result = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(amount_usdc), 0) as total_volume
            FROM bets
            WHERE event_id = $1
            "#,
            event_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.total_volume)
    }

    /// Get total volume by outcome for an event
    pub async fn get_volume_by_outcome(
        &self,
        event_id: Uuid,
    ) -> SqlxResult<Vec<(String, Decimal)>> {
        let results = sqlx::query!(
            r#"
            SELECT outcome, COALESCE(SUM(amount_usdc), 0) as volume
            FROM bets
            WHERE event_id = $1
            GROUP BY outcome
            ORDER BY volume DESC
            "#,
            event_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| (r.outcome, r.volume.unwrap_or(Decimal::ZERO)))
            .collect())
    }

    /// Get bet count for an event
    pub async fn count_by_event(&self, event_id: Uuid) -> SqlxResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM bets
            WHERE event_id = $1
            "#,
            event_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count.unwrap_or(0))
    }
}


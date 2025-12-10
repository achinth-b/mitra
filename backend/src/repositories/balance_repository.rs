//! Repository for balance and transaction operations

use crate::error::RepositoryError;
use crate::models::{Payout, Settlement, Transaction, TransactionType, UserGroupBalance};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct BalanceRepository {
    pool: PgPool,
}

impl BalanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // User Group Balance Operations
    // =========================================================================

    /// Get or create a user's balance in a group
    pub async fn get_or_create_balance(
        &self,
        user_id: Uuid,
        group_id: Uuid,
    ) -> Result<UserGroupBalance, RepositoryError> {
        // Try to get existing balance
        let existing = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(balance) = existing {
            return Ok(balance);
        }

        // Create new balance record
        let balance = sqlx::query_as!(
            UserGroupBalance,
            r#"
            INSERT INTO user_group_balances (user_id, group_id, balance_usdc, locked_usdc)
            VALUES ($1, $2, 0, 0)
            ON CONFLICT (user_id, group_id) DO UPDATE SET updated_at = NOW()
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(balance)
    }

    /// Get user balance in a group
    pub async fn get_balance(
        &self,
        user_id: Uuid,
        group_id: Uuid,
    ) -> Result<Option<UserGroupBalance>, RepositoryError> {
        let balance = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(balance)
    }

    /// Credit funds to user's balance (deposit or winnings)
    pub async fn credit_balance(
        &self,
        user_id: Uuid,
        group_id: Uuid,
        amount: Decimal,
        tx_type: TransactionType,
        event_id: Option<Uuid>,
        solana_sig: Option<&str>,
        description: Option<&str>,
    ) -> Result<UserGroupBalance, RepositoryError> {
        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Get current balance
        let current = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            FOR UPDATE
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let balance_before = current
            .as_ref()
            .map(|b| b.balance_usdc)
            .unwrap_or(Decimal::ZERO);
        let balance_after = balance_before + amount;

        // Update or create balance
        let updated = sqlx::query_as!(
            UserGroupBalance,
            r#"
            INSERT INTO user_group_balances (user_id, group_id, balance_usdc, locked_usdc)
            VALUES ($1, $2, $3, 0)
            ON CONFLICT (user_id, group_id) DO UPDATE 
            SET balance_usdc = user_group_balances.balance_usdc + $3, updated_at = NOW()
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id,
            amount
        )
        .fetch_one(&mut *tx)
        .await?;

        // Record transaction
        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (user_id, group_id, event_id, transaction_type, amount_usdc, balance_before, balance_after, solana_tx_signature, status, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'confirmed', $9)
            "#,
            user_id,
            group_id,
            event_id,
            tx_type.as_str(),
            amount,
            balance_before,
            balance_after,
            solana_sig,
            description
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated)
    }

    /// Debit funds from user's balance (withdrawal or bet)
    pub async fn debit_balance(
        &self,
        user_id: Uuid,
        group_id: Uuid,
        amount: Decimal,
        tx_type: TransactionType,
        event_id: Option<Uuid>,
        solana_sig: Option<&str>,
        description: Option<&str>,
    ) -> Result<UserGroupBalance, RepositoryError> {
        let mut tx = self.pool.begin().await?;

        // Get current balance with lock
        let current = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            FOR UPDATE
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound("Balance not found".to_string()))?;

        // Check sufficient balance
        let available = current.balance_usdc - current.locked_usdc;
        if available < amount {
            return Err(RepositoryError::BusinessRule(format!(
                "Insufficient balance: available {}, required {}",
                available, amount
            )));
        }

        let balance_after = current.balance_usdc - amount;

        // Update balance
        let updated = sqlx::query_as!(
            UserGroupBalance,
            r#"
            UPDATE user_group_balances 
            SET balance_usdc = $3, updated_at = NOW()
            WHERE user_id = $1 AND group_id = $2
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id,
            balance_after
        )
        .fetch_one(&mut *tx)
        .await?;

        // Record transaction
        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (user_id, group_id, event_id, transaction_type, amount_usdc, balance_before, balance_after, solana_tx_signature, status, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'confirmed', $9)
            "#,
            user_id,
            group_id,
            event_id,
            tx_type.as_str(),
            amount,
            current.balance_usdc,
            balance_after,
            solana_sig,
            description
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated)
    }

    /// Lock funds for a bet (moves from available to locked)
    pub async fn lock_for_bet(
        &self,
        user_id: Uuid,
        group_id: Uuid,
        amount: Decimal,
        event_id: Uuid,
    ) -> Result<UserGroupBalance, RepositoryError> {
        let mut tx = self.pool.begin().await?;

        // Get current balance with lock
        let current = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            FOR UPDATE
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound("Balance not found".to_string()))?;

        // Check sufficient available balance
        let available = current.balance_usdc - current.locked_usdc;
        if available < amount {
            return Err(RepositoryError::BusinessRule(format!(
                "Insufficient available balance: {} available, {} required",
                available, amount
            )));
        }

        // Increase locked amount
        let updated = sqlx::query_as!(
            UserGroupBalance,
            r#"
            UPDATE user_group_balances 
            SET locked_usdc = locked_usdc + $3, updated_at = NOW()
            WHERE user_id = $1 AND group_id = $2
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id,
            amount
        )
        .fetch_one(&mut *tx)
        .await?;

        // Record the bet transaction
        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (user_id, group_id, event_id, transaction_type, amount_usdc, balance_before, balance_after, status, description)
            VALUES ($1, $2, $3, 'bet_placed', $4, $5, $5, 'confirmed', 'Bet placed - funds locked')
            "#,
            user_id,
            group_id,
            event_id,
            amount,
            current.balance_usdc
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated)
    }

    /// Unlock and deduct funds when bet resolves to loss
    pub async fn settle_loss(
        &self,
        user_id: Uuid,
        group_id: Uuid,
        amount: Decimal,
        event_id: Uuid,
    ) -> Result<UserGroupBalance, RepositoryError> {
        let mut tx = self.pool.begin().await?;

        let current = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            FOR UPDATE
            "#,
            user_id,
            group_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let balance_after = current.balance_usdc - amount;
        let locked_after = current.locked_usdc - amount;

        let updated = sqlx::query_as!(
            UserGroupBalance,
            r#"
            UPDATE user_group_balances 
            SET balance_usdc = $3, locked_usdc = $4, updated_at = NOW()
            WHERE user_id = $1 AND group_id = $2
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id,
            balance_after,
            locked_after.max(Decimal::ZERO)
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (user_id, group_id, event_id, transaction_type, amount_usdc, balance_before, balance_after, status, description)
            VALUES ($1, $2, $3, 'bet_lost', $4, $5, $6, 'confirmed', 'Bet lost - funds deducted')
            "#,
            user_id,
            group_id,
            event_id,
            amount,
            current.balance_usdc,
            balance_after
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated)
    }

    /// Unlock funds and add winnings when bet resolves to win
    pub async fn settle_win(
        &self,
        user_id: Uuid,
        group_id: Uuid,
        original_bet: Decimal,
        winnings: Decimal,
        event_id: Uuid,
    ) -> Result<UserGroupBalance, RepositoryError> {
        let mut tx = self.pool.begin().await?;

        let current = sqlx::query_as!(
            UserGroupBalance,
            r#"
            SELECT user_id, group_id, balance_usdc, locked_usdc, updated_at
            FROM user_group_balances
            WHERE user_id = $1 AND group_id = $2
            FOR UPDATE
            "#,
            user_id,
            group_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Balance goes up by winnings (original bet was locked, not deducted)
        // Unlock the original bet and add winnings
        let balance_after = current.balance_usdc + winnings;
        let locked_after = (current.locked_usdc - original_bet).max(Decimal::ZERO);

        let updated = sqlx::query_as!(
            UserGroupBalance,
            r#"
            UPDATE user_group_balances 
            SET balance_usdc = $3, locked_usdc = $4, updated_at = NOW()
            WHERE user_id = $1 AND group_id = $2
            RETURNING user_id, group_id, balance_usdc, locked_usdc, updated_at
            "#,
            user_id,
            group_id,
            balance_after,
            locked_after
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (user_id, group_id, event_id, transaction_type, amount_usdc, balance_before, balance_after, status, description)
            VALUES ($1, $2, $3, 'bet_won', $4, $5, $6, 'confirmed', 'Bet won - winnings credited')
            "#,
            user_id,
            group_id,
            event_id,
            winnings,
            current.balance_usdc,
            balance_after
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated)
    }

    // =========================================================================
    // Transaction History
    // =========================================================================

    /// Get transaction history for a user
    pub async fn get_user_transactions(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT id, user_id, group_id, event_id, transaction_type, amount_usdc,
                   balance_before, balance_after, solana_tx_signature, status, description, created_at
            FROM transactions
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }

    // =========================================================================
    // Settlement Operations
    // =========================================================================

    /// Create a settlement record
    pub async fn create_settlement(
        &self,
        event_id: Uuid,
        winning_outcome: &str,
        total_pool: Decimal,
        total_winning_shares: Decimal,
        settled_by_wallet: &str,
        solana_sig: Option<&str>,
    ) -> Result<Settlement, RepositoryError> {
        let settlement = sqlx::query_as!(
            Settlement,
            r#"
            INSERT INTO settlements 
            (event_id, winning_outcome, total_pool, total_winning_shares, settled_by_wallet, solana_tx_signature)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, event_id, winning_outcome, total_pool, total_winning_shares, settled_by_wallet, solana_tx_signature, settled_at
            "#,
            event_id,
            winning_outcome,
            total_pool,
            total_winning_shares,
            settled_by_wallet,
            solana_sig
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(settlement)
    }

    /// Create a payout record for a winner
    pub async fn create_payout(
        &self,
        settlement_id: Uuid,
        user_id: Uuid,
        shares: Decimal,
        payout_amount: Decimal,
    ) -> Result<Payout, RepositoryError> {
        let payout = sqlx::query_as!(
            Payout,
            r#"
            INSERT INTO payouts (settlement_id, user_id, shares, payout_amount)
            VALUES ($1, $2, $3, $4)
            RETURNING id, settlement_id, user_id, shares, payout_amount, claimed, claimed_at, solana_tx_signature, created_at
            "#,
            settlement_id,
            user_id,
            shares,
            payout_amount
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(payout)
    }

    /// Get unclaimed payouts for a user
    pub async fn get_unclaimed_payouts(&self, user_id: Uuid) -> Result<Vec<Payout>, RepositoryError> {
        let payouts = sqlx::query_as!(
            Payout,
            r#"
            SELECT id, settlement_id, user_id, shares, payout_amount, claimed, claimed_at, solana_tx_signature, created_at
            FROM payouts
            WHERE user_id = $1 AND claimed = FALSE
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(payouts)
    }

    /// Mark a payout as claimed
    pub async fn mark_payout_claimed(
        &self,
        payout_id: Uuid,
        solana_sig: &str,
    ) -> Result<Payout, RepositoryError> {
        let payout = sqlx::query_as!(
            Payout,
            r#"
            UPDATE payouts 
            SET claimed = TRUE, claimed_at = NOW(), solana_tx_signature = $2
            WHERE id = $1
            RETURNING id, settlement_id, user_id, shares, payout_amount, claimed, claimed_at, solana_tx_signature, created_at
            "#,
            payout_id,
            solana_sig
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(payout)
    }
}


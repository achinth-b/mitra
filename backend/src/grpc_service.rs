use crate::amm::LmsrAmm;
use crate::auth;
use crate::error::{AppError, AppResult};
use crate::models::{Event, EventStatus, SettlementType};
use crate::repositories::*;
use crate::state_manager::StateManager;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// gRPC service implementation
/// 
/// Note: This is a skeleton implementation. The actual gRPC code will be generated
/// from the proto file using tonic-build. This module provides the business logic.
pub struct MitraService {
    app_state: Arc<crate::AppState>,
    state_manager: Arc<StateManager>,
}

impl MitraService {
    /// Create a new gRPC service
    pub fn new(app_state: Arc<crate::AppState>, state_manager: Arc<StateManager>) -> Self {
        Self {
            app_state,
            state_manager,
        }
    }

    /// Create a friend group
    pub async fn create_friend_group(
        &self,
        name: String,
        admin_wallet: String,
        solana_pubkey: String,
        signature: String,
    ) -> AppResult<(Uuid, String)> {
        // Verify signature
        auth::verify_auth_with_timestamp(
            &admin_wallet,
            "create_group",
            chrono::Utc::now().timestamp(),
            &signature,
        )?;

        // Create group in database
        let group = self
            .app_state
            .friend_group_repo
            .create(&solana_pubkey, &name, &admin_wallet)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        info!("Created friend group: {} ({})", group.id, group.name);

        Ok((group.id, solana_pubkey))
    }

    /// Invite a member to a group
    pub async fn invite_member(
        &self,
        group_id: Uuid,
        invited_wallet: String,
        inviter_wallet: String,
        signature: String,
    ) -> AppResult<Uuid> {
        // Verify signature
        auth::verify_auth_with_timestamp(
            &inviter_wallet,
            "invite_member",
            chrono::Utc::now().timestamp(),
            &signature,
        )?;

        // Verify inviter is admin
        let role = self
            .app_state
            .group_member_repo
            .find_role(group_id, Uuid::new_v4()) // TODO: Get user_id from wallet
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // TODO: Check if inviter is admin

        // Find or create user
        let user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&invited_wallet)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Add member
        let member = self
            .app_state
            .group_member_repo
            .add_member(group_id, user.id, crate::models::MemberRole::Member)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        Ok(member.user_id)
    }

    /// Create an event
    pub async fn create_event(
        &self,
        group_id: Uuid,
        title: String,
        description: Option<String>,
        outcomes: Vec<String>,
        settlement_type: SettlementType,
        resolve_by: Option<chrono::NaiveDateTime>,
        creator_wallet: String,
        signature: String,
    ) -> AppResult<Event> {
        // Verify signature
        auth::verify_auth_with_timestamp(
            &creator_wallet,
            "create_event",
            chrono::Utc::now().timestamp(),
            &signature,
        )?;

        // Verify creator is group member
        // TODO: Implement member check

        // Create event
        let outcomes_json = serde_json::to_value(outcomes.clone())
            .map_err(|e| AppError::Serialization(e))?;

        let event = self
            .app_state
            .event_repo
            .create(
                group_id,
                &title,
                description.as_deref(),
                &outcomes_json,
                &settlement_type.as_str().to_string(),
                resolve_by,
            )
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        info!("Created event: {} ({})", event.id, event.title);

        Ok(event)
    }

    /// Place a bet
    pub async fn place_bet(
        &self,
        event_id: Uuid,
        user_wallet: String,
        outcome: String,
        amount_usdc: Decimal,
        signature: String,
    ) -> AppResult<(Uuid, Decimal, Decimal, HashMap<String, Decimal>)> {
        // Verify signature
        auth::verify_auth_with_timestamp(
            &user_wallet,
            "place_bet",
            chrono::Utc::now().timestamp(),
            &signature,
        )?;

        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Verify event is active
        if !event.is_active() {
            return Err(AppError::BusinessLogic("Event is not active".to_string()));
        }

        // Verify outcome is valid
        let outcomes = event.outcomes_vec();
        if !outcomes.contains(&outcome) {
            return Err(AppError::Validation(format!("Invalid outcome: {}", outcome)));
        }

        // Find or create user
        let user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&user_wallet)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Get current bets to calculate AMM state
        let bets = self
            .app_state
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Initialize AMM with current state
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), outcomes.clone())
            .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))?;

        // Update AMM with existing shares
        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))?;
        }

        // Calculate buy
        let (shares, price, new_prices) = amm
            .calculate_buy(&outcome, amount_usdc)
            .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))?;

        // Create bet
        let bet = self
            .app_state
            .bet_repo
            .create(event_id, user.id, &outcome, shares, price, amount_usdc)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        info!(
            "Bet placed: {} on event {} for {} shares at price {}",
            bet.id, event_id, shares, price
        );

        Ok((bet.id, shares, price, new_prices))
    }

    /// Get event prices
    pub async fn get_event_prices(
        &self,
        event_id: Uuid,
    ) -> AppResult<HashMap<String, Decimal>> {
        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Get bets
        let bets = self
            .app_state
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Initialize AMM
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), event.outcomes_vec())
            .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))?;

        // Update with existing shares
        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))?;
        }

        // Get prices
        amm.get_prices()
            .map_err(|e| AppError::BusinessLogic(format!("AMM error: {}", e)))
    }

    /// Settle an event
    pub async fn settle_event(
        &self,
        event_id: Uuid,
        winning_outcome: String,
        settler_wallet: String,
        signature: String,
    ) -> AppResult<String> {
        // Verify signature
        auth::verify_auth_with_timestamp(
            &settler_wallet,
            "settle_event",
            chrono::Utc::now().timestamp(),
            &signature,
        )?;

        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Verify outcome is valid
        let outcomes = event.outcomes_vec();
        if !outcomes.contains(&winning_outcome) {
            return Err(AppError::Validation(format!("Invalid outcome: {}", winning_outcome)));
        }

        // Update status
        self.app_state
            .event_repo
            .update_status(event_id, EventStatus::Resolved)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // TODO: Call Solana program to settle on-chain
        // For MVP, return placeholder
        let tx_signature = "placeholder_settle_tx".to_string();

        info!("Event {} settled with outcome: {}", event_id, winning_outcome);

        Ok(tx_signature)
    }
}

use std::collections::HashMap;


use crate::amm::LmsrAmm;
use crate::auth;
use crate::error::{AppError, AppResult};
use crate::models::Event;
use crate::repositories::{BetRepository, EventRepository, GroupMemberRepository, UserRepository};
use crate::services::SettlementService;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Service for managing events
pub struct EventService {
    event_repo: Arc<EventRepository>,
    user_repo: Arc<UserRepository>,
    member_repo: Arc<GroupMemberRepository>,
    bet_repo: Arc<BetRepository>,
    settlement_service: Arc<SettlementService>,
}

pub struct EventPrices {
    pub prices: std::collections::HashMap<String, f64>,
    pub total_volume: f64,
}

impl EventService {
    pub fn new(
        event_repo: Arc<EventRepository>,
        user_repo: Arc<UserRepository>,
        member_repo: Arc<GroupMemberRepository>,
        bet_repo: Arc<BetRepository>,
        settlement_service: Arc<SettlementService>,
    ) -> Self {
        Self {
            event_repo,
            user_repo,
            member_repo,
            bet_repo,
            settlement_service,
        }
    }

    /// Create a new event
    pub async fn create_event(
        &self,
        group_id: Uuid,
        title: &str,
        description: Option<&str>,
        outcomes: &[String],
        settlement_type: &str,
        resolve_by: Option<i64>,
        creator_wallet: &str,
        arbiter_wallet: Option<&str>,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<Event> {
        info!("Creating event: group={}, title={}", group_id, title);

        // Verify signature
        auth::verify_auth_with_timestamp(creator_wallet, "create_event", timestamp, signature)?;

        // Verify creator is member
        let creator_user = self.user_repo.find_or_create_by_wallet(creator_wallet).await?;
        if !self
            .member_repo
            .is_member(group_id, creator_user.id)
            .await
            .map_err(|e| AppError::Database(e.into()))?
        {
            return Err(AppError::Unauthorized(
                "Only group members can create events".into(),
            ));
        }

        // Validate outcomes
        if outcomes.len() < 2 {
            return Err(AppError::Validation("At least 2 outcomes required".into()));
        }

        // Prepare data
        let outcomes_json = serde_json::to_value(outcomes)
            .map_err(|e| AppError::Validation(format!("Serialization error: {}", e)))?;

        let resolve_by_dt = resolve_by.and_then(|ts| {
            if ts > 0 {
                chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.naive_utc())
            } else {
                None
            }
        });

        // Generate keypair (Simulated for PoC)
        let event_keypair = Keypair::new();
        let solana_pubkey = event_keypair.pubkey().to_string();

        // Validate arbiter
        let arbiter = if settlement_type == "manual" {
            arbiter_wallet
        } else {
            None
        };

        // Create in DB
        let event = self
            .event_repo
            .create(
                group_id,
                title,
                description,
                &outcomes_json,
                settlement_type,
                resolve_by_dt,
                Some(&solana_pubkey),
                arbiter,
            )
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        info!("Created event {} ({})", event.title, event.id);
        Ok(event)
    }

    /// Get all events for a group
    pub async fn get_group_events(&self, group_id: Uuid) -> AppResult<Vec<Event>> {
        self.event_repo
            .find_by_group(group_id)
            .await
            .map_err(|e| AppError::Database(e.into()))
    }

    /// Get prices for an event
    pub async fn get_event_prices(&self, event_id: Uuid) -> AppResult<EventPrices> {
        let event = self
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

        let bets = self
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        // AMM Calc
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), event.outcomes_vec())
            .map_err(|e| AppError::Message(format!("AMM error: {}", e)))?;

        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| AppError::Message(format!("AMM error: {}", e)))?;
        }

        let prices = amm
            .get_prices()
            .map_err(|e| AppError::Message(format!("AMM error: {}", e)))?;

        let prices_f64 = prices
            .iter()
            .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
            .collect();

        let total_volume = bets
            .iter()
            .map(|b| b.amount_usdc.to_f64().unwrap_or(0.0))
            .sum();

        Ok(EventPrices {
            prices: prices_f64,
            total_volume,
        })
    }

    /// Delete event
    pub async fn delete_event(
        &self,
        event_id: Uuid,
        deleter_wallet: &str,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<bool> {
        auth::verify_auth_with_timestamp(deleter_wallet, "delete_event", timestamp, signature)?;

        let event = self
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

        let user = self.user_repo.find_or_create_by_wallet(deleter_wallet).await?;

        // Check if creator or admin
        // Note: Logic simplified here, assuming only admin for now based on previous code
        let role = self
            .member_repo
            .find_role(event.group_id, user.id)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        let is_admin = matches!(role, Some(crate::models::MemberRole::Admin));
        
        // Also allow creator of event if needed, but schema doesn't strictly link creator ID easily without parsing
        // For safety, let's stick to admin only or if wallets match
        
        if !is_admin {
             return Err(AppError::Unauthorized("Only admin can delete events".into()));
        }

        let success = self.event_repo.delete(event_id).await.map_err(|e| AppError::Database(e.into()))?;
        Ok(success)
    }

    /// Settle event
    pub async fn settle_event(
        &self,
        event_id: Uuid,
        winning_outcome: &str,
        settler_wallet: &str,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<String> {
        auth::verify_auth_with_timestamp(settler_wallet, "settle_event", timestamp, signature)?;

        // Delegate to settlement service which handles verification and execution
        self.settlement_service
            .settle_manual(event_id, winning_outcome.to_string(), settler_wallet.to_string())
            .await
    }
}

//! gRPC service implementation for Mitra
//!
//! This module implements the MitraService gRPC handlers using tonic.
//! The proto definitions are compiled at build time via build.rs.

use crate::amm::LmsrAmm;
use crate::auth;
use crate::error::{AppError, AppResult};
use crate::models::EventStatus;
use crate::state_manager::StateManager;
use anchor_client::solana_sdk;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

// Include the generated proto code
// Falls back to stub implementation if protoc is not available
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/mitra.rs"));
    
    /// File descriptor set for gRPC reflection
    pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mitra_descriptor.bin"));
}

use proto::mitra_service_server::{MitraService, MitraServiceServer};
use proto::{
    CreateGroupRequest, GroupResponse, InviteMemberRequest, MemberResponse,
    CreateEventRequest, EventResponse, PlaceBetRequest, BetResponse,
    GetPricesRequest, PricesResponse, SettleEventRequest, SettleResponse,
    DeleteEventRequest, DeleteEventResponse,
    DepositRequest, DepositResponse, WithdrawRequest, WithdrawResponse,
    BalanceRequest, BalanceResponse, ClaimRequest, ClaimResponse,
};

/// gRPC service implementation
pub struct MitraGrpcService {
    app_state: Arc<crate::AppState>,
    state_manager: Arc<StateManager>,
}

impl MitraGrpcService {
    /// Create a new gRPC service
    pub fn new(app_state: Arc<crate::AppState>, state_manager: Arc<StateManager>) -> Self {
        Self {
            app_state,
            state_manager,
        }
    }

    /// Create a tonic server for this service
    pub fn into_server(self) -> MitraServiceServer<Self> {
        MitraServiceServer::new(self)
    }

    /// Convert AppError to tonic Status
    fn to_status(err: AppError) -> Status {
        match err {
            AppError::NotFound(msg) => Status::not_found(msg),
            AppError::Unauthorized(msg) => Status::unauthenticated(msg),
            AppError::Validation(msg) => Status::invalid_argument(msg),
            AppError::BusinessLogic(msg) => Status::failed_precondition(msg),
            AppError::Database(_) | AppError::Sqlx(_) => {
                error!("Database error: {:?}", err);
                Status::internal("Database error")
            }
            _ => {
                error!("Internal error: {:?}", err);
                Status::internal("Internal server error")
            }
        }
    }

    /// Helper to parse UUID from string
    fn parse_uuid(s: &str, field_name: &str) -> Result<Uuid, Status> {
        Uuid::parse_str(s)
            .map_err(|_| Status::invalid_argument(format!("Invalid {}: {}", field_name, s)))
    }
}

#[tonic::async_trait]
impl MitraService for MitraGrpcService {
    /// Create a friend group
    async fn create_friend_group(
        &self,
        request: Request<CreateGroupRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        let req = request.into_inner();
        info!("CreateFriendGroup request: name={}", req.name);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.admin_wallet,
            "create_group",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Create group in database
        let group = self
            .app_state
            .friend_group_repo
            .create(&req.solana_pubkey, &req.name, &req.admin_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Add admin as first member
        let admin_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.admin_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        self.app_state
            .group_member_repo
            .add_member(group.id, admin_user.id, crate::models::MemberRole::Admin)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("Created friend group: {} ({})", group.id, group.name);

        Ok(Response::new(GroupResponse {
            group_id: group.id.to_string(),
            solana_pubkey: group.solana_pubkey,
            name: group.name,
            admin_wallet: group.admin_wallet,
            created_at: group.created_at.and_utc().timestamp(),
        }))
    }

    /// Invite a member to a group
    async fn invite_member(
        &self,
        request: Request<InviteMemberRequest>,
    ) -> Result<Response<MemberResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        info!("InviteMember request: group={}, invited={}", group_id, req.invited_wallet);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.inviter_wallet,
            "invite_member",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Verify inviter is admin or member
        let inviter_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.inviter_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let is_member = self
            .app_state
            .group_member_repo
            .is_member(group_id, inviter_user.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if !is_member {
            return Err(Status::permission_denied("Only group members can invite"));
        }

        // Find or create invited user
        let invited_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.invited_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Add as member
        let member = self
            .app_state
            .group_member_repo
            .add_member(group_id, invited_user.id, crate::models::MemberRole::Member)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("Added member {} to group {}", invited_user.id, group_id);

        Ok(Response::new(MemberResponse {
            group_id: group_id.to_string(),
            user_id: invited_user.id.to_string(),
            wallet_address: invited_user.wallet_address,
            role: "member".to_string(),
            joined_at: member.joined_at.and_utc().timestamp(),
        }))
    }

    /// Create an event
    async fn create_event(
        &self,
        request: Request<CreateEventRequest>,
    ) -> Result<Response<EventResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        info!("CreateEvent request: group={}, title={}", group_id, req.title);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.creator_wallet,
            "create_event",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Verify creator is group member
        let creator_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.creator_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let is_member = self
            .app_state
            .group_member_repo
            .is_member(group_id, creator_user.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if !is_member {
            return Err(Status::permission_denied("Only group members can create events"));
        }

        // Validate outcomes
        if req.outcomes.len() < 2 {
            return Err(Status::invalid_argument("At least 2 outcomes required"));
        }

        // Parse resolve_by timestamp
        let resolve_by = if req.resolve_by > 0 {
            Some(chrono::DateTime::from_timestamp(req.resolve_by, 0)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| Status::invalid_argument("Invalid resolve_by timestamp"))?)
        } else {
            None
        };

        // Create event
        let outcomes_json = serde_json::to_value(&req.outcomes)
            .map_err(|e| Status::internal(format!("Serialization error: {}", e)))?;

        // Generate a random Solana keypair for the event account
        // In a real implementation this would be derived or input from the client
        // after the client creates the account on-chain
        let event_keypair = anchor_client::solana_sdk::signature::Keypair::new();
        let solana_pubkey = anchor_client::solana_sdk::signer::Signer::pubkey(&event_keypair).to_string();

        // Validate arbiter if settlement type is manual
        let arbiter_wallet = if req.settlement_type == "manual" && !req.arbiter_wallet.is_empty() {
             Some(req.arbiter_wallet.as_str())
        } else {
             None
        };

        let event = self
            .app_state
            .event_repo
            .create(
                group_id,
                &req.title,
                Some(&req.description),
                &outcomes_json,
                &req.settlement_type,
                resolve_by,
                Some(&solana_pubkey),
                arbiter_wallet,
            )
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("Created event: {} ({})", event.id, event.title);

        let description = event.description.clone().unwrap_or_default();
        let outcomes = event.outcomes_vec();
        
        Ok(Response::new(EventResponse {
            event_id: event.id.to_string(),
            group_id: event.group_id.to_string(),
            solana_pubkey: event.solana_pubkey.unwrap_or_default(),
            title: event.title,
            description,
            outcomes,
            settlement_type: event.settlement_type,
            status: event.status,
            resolve_by: event.resolve_by.map(|dt| dt.and_utc().timestamp()).unwrap_or(0),
            created_at: event.created_at.and_utc().timestamp(),
            arbiter_wallet: event.arbiter_wallet.unwrap_or_default(),
        }))
    }

    /// Place a bet
    async fn place_bet(
        &self,
        request: Request<PlaceBetRequest>,
    ) -> Result<Response<BetResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        info!("PlaceBet request: event={}, outcome={}, amount={}", event_id, req.outcome, req.amount_usdc);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.user_wallet,
            "place_bet",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Event {} not found", event_id)))?;

        // Verify event is active
        if !event.is_active() {
            return Err(Status::failed_precondition("Event is not active"));
        }

        // Verify outcome is valid
        let outcomes = event.outcomes_vec();
        if !outcomes.contains(&req.outcome) {
            return Err(Status::invalid_argument(format!("Invalid outcome: {}", req.outcome)));
        }

        // Find or create user
        let user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Convert amount to Decimal
        let amount_usdc = Decimal::try_from(req.amount_usdc)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        if amount_usdc <= Decimal::ZERO {
            return Err(Status::invalid_argument("Amount must be positive"));
        }

        // Check user balance in this group
        let balance = self
            .app_state
            .balance_repo
            .get_or_create_balance(user.id, event.group_id)
            .await
            .map_err(|e| Status::internal(format!("Balance error: {}", e)))?;

        let available = balance.balance_usdc - balance.locked_usdc;
        if available < amount_usdc {
            return Err(Status::failed_precondition(format!(
                "Insufficient balance: available {} USDC, required {} USDC",
                available, amount_usdc
            )));
        }

        // Get current bets to calculate AMM state
        let bets = self
            .app_state
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Initialize AMM with current state
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), outcomes.clone())
            .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;

        // Update AMM with existing shares
        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;
        }

        // Calculate buy
        let (shares, price, new_prices) = amm
            .calculate_buy(&req.outcome, amount_usdc)
            .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;

        // Lock funds for this bet
        self.app_state
            .balance_repo
            .lock_for_bet(user.id, event.group_id, amount_usdc, event_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to lock funds: {}", e)))?;

        // Create bet record
        let bet = self
            .app_state
            .bet_repo
            .create(event_id, user.id, &req.outcome, shares, price, amount_usdc)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!(
            "Bet placed: {} on event {} for {} shares at price {} (locked {} USDC)",
            bet.id, event_id, shares, price, amount_usdc
        );

        // Convert prices to f64 for response
        let prices_f64: HashMap<String, f64> = new_prices
            .iter()
            .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
            .collect();

        Ok(Response::new(BetResponse {
            bet_id: bet.id.to_string(),
            shares: shares.to_f64().unwrap_or(0.0),
            price: price.to_f64().unwrap_or(0.0),
            updated_prices: Some(PricesResponse {
                event_id: event_id.to_string(),
                prices: prices_f64,
                total_volume: bets.iter().map(|b| b.amount_usdc.to_f64().unwrap_or(0.0)).sum::<f64>() + req.amount_usdc,
                timestamp: chrono::Utc::now().timestamp(),
            }),
        }))
    }

    /// Get event prices
    async fn get_event_prices(
        &self,
        request: Request<GetPricesRequest>,
    ) -> Result<Response<PricesResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Event {} not found", event_id)))?;

        // Get bets
        let bets = self
            .app_state
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Initialize AMM
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), event.outcomes_vec())
            .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;

        // Update with existing shares
        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;
        }

        // Get prices
        let prices = amm
            .get_prices()
            .map_err(|e| Status::internal(format!("AMM error: {}", e)))?;

        // Convert to f64
        let prices_f64: HashMap<String, f64> = prices
            .iter()
            .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
            .collect();

        let total_volume: f64 = bets
            .iter()
            .map(|b| b.amount_usdc.to_f64().unwrap_or(0.0))
            .sum();

        Ok(Response::new(PricesResponse {
            event_id: event_id.to_string(),
            prices: prices_f64,
            total_volume,
            timestamp: chrono::Utc::now().timestamp(),
        }))
    }

    /// Settle an event
    async fn settle_event(
        &self,
        request: Request<SettleEventRequest>,
    ) -> Result<Response<SettleResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        info!("SettleEvent request: event={}, outcome={}", event_id, req.winning_outcome);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.settler_wallet,
            "settle_event",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Get event
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Event {} not found", event_id)))?;

        // Verify event is active
        if !event.is_active() {
            return Err(Status::failed_precondition("Event is already settled or cancelled"));
        }

        // Verify settler is admin of the group
        let settler_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.settler_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let role = self
            .app_state
            .group_member_repo
            .find_role(event.group_id, settler_user.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match role {
            Some(crate::models::MemberRole::Admin) => {}
            _ => return Err(Status::permission_denied("Only admins can settle events")),
        }

        // Verify outcome is valid
        let outcomes = event.outcomes_vec();
        if !outcomes.contains(&req.winning_outcome) {
            return Err(Status::invalid_argument(format!(
                "Invalid outcome: {}. Valid outcomes: {:?}",
                req.winning_outcome, outcomes
            )));
        }

        // Get all bets for this event
        let bets = self
            .app_state
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Calculate total pool and winning shares
        let total_pool: Decimal = bets.iter().map(|b| b.amount_usdc).sum();
        let winning_bets: Vec<_> = bets.iter().filter(|b| b.outcome == req.winning_outcome).collect();
        let total_winning_shares: Decimal = winning_bets.iter().map(|b| b.shares).sum();

        info!(
            "Settlement: total_pool={}, winning_outcome={}, total_winning_shares={}",
            total_pool, req.winning_outcome, total_winning_shares
        );

        // Update event status
        self.app_state
            .event_repo
            .update_status(event_id, EventStatus::Resolved)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Call Solana program to settle on-chain
        let tx_signature = if let Some(solana_pubkey) = &event.solana_pubkey {
            match self.app_state.solana_client.settle_event(
                solana_pubkey,
                &event.group_id.to_string(), // TODO: Get actual group solana pubkey
                &req.winning_outcome,
            ).await {
                Ok(sig) => sig,
                Err(e) => {
                    error!("Failed to settle on-chain: {}", e);
                    format!("settle_offline_{}", chrono::Utc::now().timestamp())
                }
            }
        } else {
            format!("settle_no_chain_{}", chrono::Utc::now().timestamp())
        };

        // Create settlement record
        let settlement = self
            .app_state
            .balance_repo
            .create_settlement(
                event_id,
                &req.winning_outcome,
                total_pool,
                total_winning_shares,
                &req.settler_wallet,
                Some(&tx_signature),
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to create settlement: {}", e)))?;

        // Process payouts
        // Group bets by user
        let mut user_bets: std::collections::HashMap<uuid::Uuid, Vec<&crate::models::Bet>> = std::collections::HashMap::new();
        for bet in &bets {
            user_bets.entry(bet.user_id).or_default().push(bet);
        }

        for (user_id, user_bet_list) in user_bets {
            let user_winning_bets: Vec<_> = user_bet_list.iter()
                .filter(|b| b.outcome == req.winning_outcome)
                .collect();
            
            let user_losing_bets: Vec<_> = user_bet_list.iter()
                .filter(|b| b.outcome != req.winning_outcome)
                .collect();

            // Process winning bets
            if !user_winning_bets.is_empty() {
                let user_winning_shares: Decimal = user_winning_bets.iter().map(|b| b.shares).sum();
                let original_bet_amount: Decimal = user_winning_bets.iter().map(|b| b.amount_usdc).sum();
                
                // Calculate payout: user_shares / total_winning_shares * total_pool
                let payout = if total_winning_shares > Decimal::ZERO {
                    (user_winning_shares / total_winning_shares) * total_pool
                } else {
                    original_bet_amount // Refund if no winners
                };

                let winnings = payout - original_bet_amount; // Net profit

                // Record payout
                if let Err(e) = self.app_state.balance_repo.create_payout(
                    settlement.id,
                    user_id,
                    user_winning_shares,
                    payout,
                ).await {
                    error!("Failed to create payout record for user {}: {}", user_id, e);
                }

                // Credit winnings to user balance
                if let Err(e) = self.app_state.balance_repo.settle_win(
                    user_id,
                    event.group_id,
                    original_bet_amount,
                    winnings,
                    event_id,
                ).await {
                    error!("Failed to credit winnings for user {}: {}", user_id, e);
                }
            }

            // Process losing bets
            for losing_bet in user_losing_bets {
                if let Err(e) = self.app_state.balance_repo.settle_loss(
                    user_id,
                    event.group_id,
                    losing_bet.amount_usdc,
                    event_id,
                ).await {
                    error!("Failed to process loss for user {}: {}", user_id, e);
                }
            }
        }

        info!("Event {} settled with outcome: {} ({} bets processed)", event_id, req.winning_outcome, bets.len());

        Ok(Response::new(SettleResponse {
            event_id: event_id.to_string(),
            winning_outcome: req.winning_outcome,
            settled_at: chrono::Utc::now().timestamp(),
            solana_tx_signature: tx_signature,
        }))
    }

    /// Delete an event
    async fn delete_event(
        &self,
        request: Request<DeleteEventRequest>,
    ) -> Result<Response<DeleteEventResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        info!("DeleteEvent request: event={}, deleter={}", event_id, req.deleter_wallet);

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.deleter_wallet,
            "delete_event",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Get event to check permissions
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Event {} not found", event_id)))?;

        // Verify deleter is admin or creator
        let deleter_user = self
            .app_state
            .user_repo
            .find_or_create_by_wallet(&req.deleter_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Check if admin
        let role = self
            .app_state
            .group_member_repo
            .find_role(event.group_id, deleter_user.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Allow deletion if admin OR if it's the creator (and no bets placed yet ideally, but keeping simple)
        // For now, strict: only admins can delete
        match role {
            Some(crate::models::MemberRole::Admin) => {}
            _ => return Err(Status::permission_denied("Only group admins can delete events")),
        }

        // Determine success
        let success = self
            .app_state
            .event_repo
            .delete(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if success {
            info!("Deleted event: {}", event_id);
        } else {
            return Err(Status::not_found("Event not found or could not be deleted"));
        }

        Ok(Response::new(DeleteEventResponse {
            success,
            event_id: event_id.to_string(),
        }))
    }

    // ========================================================================
    // Treasury / Funds Management
    // ========================================================================

    /// Deposit funds to group treasury
    async fn deposit_funds(
        &self,
        request: Request<DepositRequest>,
    ) -> Result<Response<DepositResponse>, Status> {
        let req = request.into_inner();
        
        info!(
            "DepositFunds request: group={}, wallet={}, sol={}, usdc={}",
            req.group_id, req.user_wallet, req.amount_sol, req.amount_usdc
        );

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.user_wallet,
            "deposit_funds",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Validate amounts
        if req.amount_sol == 0 && req.amount_usdc == 0 {
            return Err(Status::invalid_argument("Must deposit at least some SOL or USDC"));
        }

        // Parse addresses
        let user_wallet = solana_sdk::pubkey::Pubkey::from_str(&req.user_wallet)
            .map_err(|e| Status::invalid_argument(format!("Invalid user wallet: {}", e)))?;
        let user_usdc_account = solana_sdk::pubkey::Pubkey::from_str(&req.user_usdc_account)
            .map_err(|e| Status::invalid_argument(format!("Invalid USDC account: {}", e)))?;

        // Call Solana
        let tx_sig = self.app_state.solana_client
            .deposit_to_treasury(
                &req.group_id,
                &user_wallet,
                &user_usdc_account,
                req.amount_sol,
                req.amount_usdc,
            )
            .await
            .map_err(|e| Status::internal(format!("Deposit failed: {}", e)))?;

        // Get updated balance
        let balance = self.app_state.solana_client
            .get_member_balance(&req.group_id, &req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Failed to get balance: {}", e)))?
            .unwrap_or(crate::solana_client::MemberBalance {
                balance_sol: req.amount_sol,
                balance_usdc: req.amount_usdc,
                locked_funds: false,
            });

        info!("Deposit successful: {}", tx_sig);

        Ok(Response::new(DepositResponse {
            success: true,
            solana_tx_signature: tx_sig,
            new_balance_sol: balance.balance_sol,
            new_balance_usdc: balance.balance_usdc,
        }))
    }

    /// Withdraw funds from group treasury
    async fn withdraw_funds(
        &self,
        request: Request<WithdrawRequest>,
    ) -> Result<Response<WithdrawResponse>, Status> {
        let req = request.into_inner();
        
        info!(
            "WithdrawFunds request: group={}, wallet={}, sol={}, usdc={}",
            req.group_id, req.user_wallet, req.amount_sol, req.amount_usdc
        );

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.user_wallet,
            "withdraw_funds",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Validate amounts
        if req.amount_sol == 0 && req.amount_usdc == 0 {
            return Err(Status::invalid_argument("Must withdraw at least some SOL or USDC"));
        }

        // Check current balance first
        let current_balance = self.app_state.solana_client
            .get_member_balance(&req.group_id, &req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Failed to get balance: {}", e)))?;

        if let Some(bal) = &current_balance {
            if bal.locked_funds {
                return Err(Status::failed_precondition("Funds are locked due to active bets"));
            }
            if bal.balance_sol < req.amount_sol {
                return Err(Status::failed_precondition("Insufficient SOL balance"));
            }
            if bal.balance_usdc < req.amount_usdc {
                return Err(Status::failed_precondition("Insufficient USDC balance"));
            }
        } else {
            return Err(Status::not_found("Member not found in group"));
        }

        // Parse addresses
        let user_wallet = solana_sdk::pubkey::Pubkey::from_str(&req.user_wallet)
            .map_err(|e| Status::invalid_argument(format!("Invalid user wallet: {}", e)))?;
        let user_usdc_account = solana_sdk::pubkey::Pubkey::from_str(&req.user_usdc_account)
            .map_err(|e| Status::invalid_argument(format!("Invalid USDC account: {}", e)))?;

        // Call Solana
        let tx_sig = self.app_state.solana_client
            .withdraw_from_treasury(
                &req.group_id,
                &user_wallet,
                &user_usdc_account,
                req.amount_sol,
                req.amount_usdc,
            )
            .await
            .map_err(|e| Status::internal(format!("Withdrawal failed: {}", e)))?;

        // Get updated balance
        let balance = self.app_state.solana_client
            .get_member_balance(&req.group_id, &req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Failed to get balance: {}", e)))?
            .unwrap_or(crate::solana_client::MemberBalance {
                balance_sol: 0,
                balance_usdc: 0,
                locked_funds: false,
            });

        info!("Withdrawal successful: {}", tx_sig);

        Ok(Response::new(WithdrawResponse {
            success: true,
            solana_tx_signature: tx_sig,
            new_balance_sol: balance.balance_sol,
            new_balance_usdc: balance.balance_usdc,
        }))
    }

    /// Get user balance in a group
    async fn get_user_balance(
        &self,
        request: Request<BalanceRequest>,
    ) -> Result<Response<BalanceResponse>, Status> {
        let req = request.into_inner();
        
        let balance = self.app_state.solana_client
            .get_member_balance(&req.group_id, &req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Failed to get balance: {}", e)))?;

        match balance {
            Some(bal) => Ok(Response::new(BalanceResponse {
                balance_sol: bal.balance_sol,
                balance_usdc: bal.balance_usdc,
                funds_locked: bal.locked_funds,
            })),
            None => Err(Status::not_found("Member not found in group")),
        }
    }

    /// Claim winnings from a resolved event
    async fn claim_winnings(
        &self,
        request: Request<ClaimRequest>,
    ) -> Result<Response<ClaimResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        info!(
            "ClaimWinnings request: event={}, wallet={}, amount={}",
            event_id, req.user_wallet, req.amount
        );

        // Verify signature
        auth::verify_auth_with_timestamp(
            &req.user_wallet,
            "claim_winnings",
            chrono::Utc::now().timestamp(),
            &req.signature,
        )
        .map_err(Self::to_status)?;

        // Get event and verify it's resolved
        let event = self
            .app_state
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Event {} not found", event_id)))?;

        if !event.is_resolved() {
            return Err(Status::failed_precondition("Event is not yet resolved"));
        }

        // Verify user is a winner (has shares in winning outcome)
        let user = self
            .app_state
            .user_repo
            .find_by_wallet(&req.user_wallet)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found("User not found"))?;

        let user_bets = self
            .app_state
            .bet_repo
            .find_by_user(user.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let winning_outcome = event.outcomes_vec()
            .first()
            .cloned()
            .unwrap_or_default(); // TODO: Get actual winning outcome from event

        let user_winning_shares: f64 = user_bets
            .iter()
            .filter(|b| b.event_id == event_id && b.outcome == winning_outcome)
            .map(|b| b.shares.to_string().parse::<f64>().unwrap_or(0.0))
            .sum();

        if user_winning_shares <= 0.0 {
            return Err(Status::failed_precondition("No winning shares to claim"));
        }

        // Parse addresses
        let user_wallet = solana_sdk::pubkey::Pubkey::from_str(&req.user_wallet)
            .map_err(|e| Status::invalid_argument(format!("Invalid user wallet: {}", e)))?;
        let user_usdc_account = solana_sdk::pubkey::Pubkey::from_str(&req.user_usdc_account)
            .map_err(|e| Status::invalid_argument(format!("Invalid USDC account: {}", e)))?;

        // Get group pubkey from event
        let group_pubkey = event.group_id.to_string(); // TODO: Get actual Solana pubkey

        // Call Solana to claim
        let tx_sig = self.app_state.solana_client
            .claim_winnings(
                &event.solana_pubkey.unwrap_or_default(),
                &group_pubkey,
                &user_wallet,
                &user_usdc_account,
                req.amount,
            )
            .await
            .map_err(|e| Status::internal(format!("Claim failed: {}", e)))?;

        info!("Claim successful: {} for {} USDC", tx_sig, req.amount);

        Ok(Response::new(ClaimResponse {
            success: true,
            solana_tx_signature: tx_sig,
            amount_claimed: req.amount,
        }))
    }
}

// Legacy compatibility: Keep the old MitraBusinessService struct for business logic
// This can be used by other parts of the system that don't go through gRPC

/// Business logic service (non-gRPC interface)
pub struct MitraBusinessService {
    app_state: Arc<crate::AppState>,
    state_manager: Arc<StateManager>,
}

impl MitraBusinessService {
    /// Create a new service
    pub fn new(app_state: Arc<crate::AppState>, state_manager: Arc<StateManager>) -> Self {
        Self {
            app_state,
            state_manager,
        }
    }

    /// Get event prices (business logic method)
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
}

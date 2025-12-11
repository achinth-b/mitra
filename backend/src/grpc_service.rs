//! gRPC service implementation for Mitra
//!
//! This module implements the MitraService gRPC handlers using tonic.
//! The proto definitions are compiled at build time via build.rs.

use crate::error::{AppError, AppResult};
use crate::state_manager::StateManager;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
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
    DeleteGroupRequest, DeleteGroupResponse,
    DepositRequest, DepositResponse, WithdrawRequest, WithdrawResponse,
    BalanceRequest, BalanceResponse, ClaimRequest, ClaimResponse,
    GetGroupEventsRequest, EventListResponse,
};

use crate::services::{GroupService, EventService, BettingService, SettlementService};

/// gRPC service implementation
pub struct MitraGrpcService {
    app_state: Arc<crate::AppState>,
    state_manager: Arc<StateManager>,
    group_service: Arc<GroupService>,
    event_service: Arc<EventService>,
    betting_service: Arc<BettingService>,
}

impl MitraGrpcService {
    /// Create a new gRPC service
    pub fn new(
        app_state: Arc<crate::AppState>, 
        state_manager: Arc<StateManager>,
        settlement_service: Arc<SettlementService>, 
    ) -> Self {
        let group_service = Arc::new(GroupService::new(
            app_state.friend_group_repo.clone(),
            app_state.user_repo.clone(),
            app_state.group_member_repo.clone(),
        ));

        let event_service = Arc::new(EventService::new(
            app_state.event_repo.clone(),
            app_state.user_repo.clone(),
            app_state.group_member_repo.clone(),
            app_state.bet_repo.clone(),
            settlement_service.clone(),
        ));

        let betting_service = Arc::new(BettingService::new(
            app_state.bet_repo.clone(),
            app_state.event_repo.clone(),
            app_state.user_repo.clone(),
            app_state.balance_repo.clone(),
            app_state.solana_client.clone(),
        ));

        Self {
            app_state,
            state_manager,
            group_service,
            event_service,
            betting_service,
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
        
        let group = self
            .group_service
            .create_group(
                &req.name, 
                &req.admin_wallet, 
                Some(&req.solana_pubkey), 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

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

        let (invited_user, member) = self
            .group_service
            .invite_member(
                group_id, 
                &req.invited_wallet, 
                &req.inviter_wallet, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(MemberResponse {
            group_id: req.group_id,
            user_id: invited_user.id.to_string(),
            wallet_address: invited_user.wallet_address,
            role: "member".to_string(),
            joined_at: member.joined_at.and_utc().timestamp(),
        }))
    }

    /// Create a new event
    async fn create_event(
        &self,
        request: Request<CreateEventRequest>,
    ) -> Result<Response<EventResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        let event = self
            .event_service
            .create_event(
                group_id, 
                &req.title, 
                Some(&req.description), 
                &req.outcomes, 
                &req.settlement_type, 
                if req.resolve_by > 0 { Some(req.resolve_by) } else { None }, 
                &req.creator_wallet, 
                if req.arbiter_wallet.is_empty() { None } else { Some(&req.arbiter_wallet) }, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        let outcomes = event.outcomes_vec();
        Ok(Response::new(EventResponse {
            event_id: event.id.to_string(),
            group_id: event.group_id.to_string(),
            solana_pubkey: event.solana_pubkey.unwrap_or_default(),
            title: event.title,
            description: event.description.unwrap_or_default(),
            outcomes,
            settlement_type: event.settlement_type,
            status: event.status.as_str().to_string(),
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
        
        let result = self
            .betting_service
            .place_bet(
                event_id, 
                &req.user_wallet, 
                &req.outcome, 
                req.amount_usdc, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(BetResponse {
            bet_id: result.bet.id.to_string(),
            shares: result.shares,
            price: result.price,
            updated_prices: Some(PricesResponse {
                event_id: event_id.to_string(),
                prices: result.updated_prices.prices,
                total_volume: result.updated_prices.total_volume,
                timestamp: chrono::Utc::now().timestamp(),
            }),
        }))
    }

    /// Get all events for a group
    async fn get_group_events(
        &self,
        request: Request<GetGroupEventsRequest>,
    ) -> Result<Response<EventListResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;

        let events = self
            .event_service
            .get_group_events(group_id)
            .await
            .map_err(Self::to_status)?;

        // Convert to proto response
        let proto_events: Vec<EventResponse> = events
            .into_iter()
            .map(|e| EventResponse {
                event_id: e.id.to_string(),
                group_id: e.group_id.to_string(),
                solana_pubkey: e.solana_pubkey.unwrap_or_default(),
                title: e.title,
                description: e.description.unwrap_or_default(),
                outcomes: e.outcomes.as_array().unwrap_or(&vec![]).iter().map(|v| v.as_str().unwrap_or("").to_string()).collect(),
                settlement_type: e.settlement_type,
                status: e.status.as_str().to_string(),
                resolve_by: e.resolve_by.map(|dt| dt.and_utc().timestamp()).unwrap_or(0),
                created_at: e.created_at.and_utc().timestamp(),
                arbiter_wallet: e.arbiter_wallet.unwrap_or_default(),
            })
            .collect();

        Ok(Response::new(EventListResponse {
            events: proto_events,
        }))
    }

    /// Get event prices
    async fn get_event_prices(
        &self,
        request: Request<GetPricesRequest>,
    ) -> Result<Response<PricesResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        let prices = self
            .event_service
            .get_event_prices(event_id)
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(PricesResponse {
            event_id: event_id.to_string(),
            prices: prices.prices,
            total_volume: prices.total_volume,
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
        
        let tx_signature = self
            .event_service
            .settle_event(
                event_id, 
                &req.winning_outcome, 
                &req.settler_wallet, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

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
        
        let success = self
            .event_service
            .delete_event(
                event_id, 
                &req.deleter_wallet, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(DeleteEventResponse {
            success,
            message: if success { "Event deleted".to_string() } else { "Failed to delete".to_string() },
        }))
    }

    /// Delete a friend group
    /// Delete a friend group
    async fn delete_group(
        &self,
        request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        // This accepts UUID but the proto field is confusingly named group_pubkey or just group_id in recent versions.
        // The original code used group_pubkey for lookup but delete needs ID.
        // Assuming request sends ID string now based on client usage.
        
        let deleted = self
            .group_service
            .delete_group(
                group_id, 
                &req.admin_wallet, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(DeleteGroupResponse {
            success: deleted,
            message: if deleted { "Group deleted".to_string() } else { "Failed to delete group".to_string() },
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
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        let (balance, tx_sig) = self
            .betting_service
            .deposit_funds(
                group_id, 
                &req.user_wallet, 
                &req.user_usdc_account, 
                req.amount_sol, 
                req.amount_usdc, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(DepositResponse {
            success: true,
            solana_tx_signature: tx_sig,
            new_balance_sol: 0, 
            new_balance_usdc: (balance.balance_usdc * Decimal::from(1_000_000)).to_u64().unwrap_or(0),
        }))
    }

    /// Withdraw funds from group treasury
    async fn withdraw_funds(
        &self,
        request: Request<WithdrawRequest>,
    ) -> Result<Response<WithdrawResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        let (balance, tx_sig) = self
            .betting_service
            .withdraw_funds(
                group_id, 
                &req.user_wallet,
                &req.user_usdc_account, 
                req.amount_usdc, 
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(WithdrawResponse {
            success: true,
            solana_tx_signature: tx_sig,
            new_balance_sol: 0,
            new_balance_usdc: (balance.balance_usdc * Decimal::from(1_000_000)).to_u64().unwrap_or(0),
        }))
    }



    /// Get user balance in a group
    async fn get_user_balance(
        &self,
        request: Request<BalanceRequest>,
    ) -> Result<Response<BalanceResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id, "group_id")?;
        
        let (balance, _) = self
            .betting_service
            .get_user_portfolio(
                &req.user_wallet, 
                group_id
            )
            .await
            .map_err(Self::to_status)?;

        Ok(Response::new(BalanceResponse {
            balance_sol: 0,
            balance_usdc: (balance.balance_usdc * Decimal::from(1_000_000)).to_u64().unwrap_or(0),
            funds_locked: balance.locked_usdc > Decimal::ZERO,
        }))
    }

    /// Claim winnings from a resolved event
    async fn claim_winnings(
        &self,
        request: Request<ClaimRequest>,
    ) -> Result<Response<ClaimResponse>, Status> {
        let req = request.into_inner();
        let event_id = Self::parse_uuid(&req.event_id, "event_id")?;
        
        let tx_sig = self
            .betting_service
            .claim_winnings(
                &req.user_wallet, 
                event_id, 
                &req.user_usdc_account,
                req.amount,
                &req.signature, 
                chrono::Utc::now().timestamp()
            )
            .await
            .map_err(Self::to_status)?;

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



}

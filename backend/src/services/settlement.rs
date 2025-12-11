use crate::error::{AppError, AppResult};
use crate::models::{Event, EventStatus};
use crate::repositories::{BalanceRepository, BetRepository, EventRepository, GroupMemberRepository};
use crate::solana_client::SolanaClient;
use crate::websocket::WebSocketServer;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Vote for consensus settlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementVote {
    pub event_id: Uuid,
    pub voter_wallet: String,
    pub winning_outcome: String,
    pub timestamp: i64,
}

/// Settlement service for handling event settlements
pub struct SettlementService {
    event_repo: Arc<EventRepository>,
    bet_repo: Arc<BetRepository>,
    group_member_repo: Arc<GroupMemberRepository>,
    balance_repo: Arc<BalanceRepository>,
    solana_client: Arc<SolanaClient>,
    ws_server: Arc<WebSocketServer>,
    pool: PgPool,
    /// Consensus votes: event_id -> votes
    consensus_votes: Arc<tokio::sync::RwLock<HashMap<Uuid, Vec<SettlementVote>>>>,
}

impl SettlementService {
    /// Create a new settlement service
    pub fn new(
        event_repo: Arc<EventRepository>,
        bet_repo: Arc<BetRepository>,
        group_member_repo: Arc<GroupMemberRepository>,
        balance_repo: Arc<BalanceRepository>,
        solana_client: Arc<SolanaClient>,
        ws_server: Arc<WebSocketServer>,
        pool: PgPool,
    ) -> Self {
        Self {
            event_repo,
            bet_repo,
            group_member_repo,
            balance_repo,
            solana_client,
            ws_server,
            pool,
            consensus_votes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Settle an event manually (admin)
    pub async fn settle_manual(
        &self,
        event_id: Uuid,
        winning_outcome: String,
        settler_wallet: String,
    ) -> AppResult<String> {
        info!("Manual settlement initiated for event {} by {}", event_id, settler_wallet);

        // Get event
        let event = self.event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Verify settler is admin
        let is_admin = self.verify_settler_permission(&event, &settler_wallet).await?;
        if !is_admin {
            return Err(AppError::Unauthorized("Only admins can manually settle events".to_string()));
        }

        // Perform settlement
        self.execute_settlement(&event, &winning_outcome, Some(&settler_wallet)).await
    }

    /// Settle an event via oracle
    pub async fn settle_oracle(
        &self,
        event_id: Uuid,
        oracle_data: HashMap<String, String>, // Oracle-specific data
    ) -> AppResult<String> {
        info!("Oracle settlement initiated for event {}", event_id);

        // Get event
        let event = self.event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Determine winning outcome from oracle data
        // TODO: Implement oracle-specific logic (Switchboard, Pyth, etc.)
        let winning_outcome = self.determine_outcome_from_oracle(&event, &oracle_data).await?;

        // Perform settlement
        self.execute_settlement(&event, &winning_outcome, None).await
    }

    /// Submit a vote for consensus settlement
    pub async fn submit_consensus_vote(
        &self,
        event_id: Uuid,
        voter_wallet: String,
        winning_outcome: String,
    ) -> AppResult<bool> {
        info!("Consensus vote submitted for event {} by {}", event_id, voter_wallet);

        // Get event
        let event = self.event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_id)))?;

        // Verify voter is group member
        let user = self.get_user_by_wallet(&voter_wallet).await?;
        let is_member = self.group_member_repo
            .is_member(event.group_id, user.id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        if !is_member {
            return Err(AppError::Unauthorized("Only group members can vote".to_string()));
        }

        // Verify outcome is valid
        let outcomes = event.outcomes_vec();
        if !outcomes.contains(&winning_outcome) {
            return Err(AppError::Validation(format!("Invalid outcome: {}", winning_outcome)));
        }

        // Add vote
        let vote = SettlementVote {
            event_id,
            voter_wallet: voter_wallet.clone(),
            winning_outcome: winning_outcome.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let mut votes = self.consensus_votes.write().await;
        let event_votes = votes.entry(event_id).or_insert_with(Vec::new);
        
        // Check if user already voted
        if event_votes.iter().any(|v| v.voter_wallet == voter_wallet) {
            return Err(AppError::BusinessLogic("User has already voted".to_string()));
        }

        event_votes.push(vote);

        // Check if threshold reached (2/3 majority)
        let member_count = self.group_member_repo
            .count_by_group(event.group_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        let threshold = (member_count * 2) / 3; // 2/3 majority
        let vote_count = event_votes.len() as i64;

        if vote_count >= threshold {
            // Determine winning outcome by majority vote
            let mut outcome_counts: HashMap<String, i64> = HashMap::new();
            for vote in event_votes.iter() {
                *outcome_counts.entry(vote.winning_outcome.clone()).or_insert(0) += 1;
            }

            let winning_outcome = outcome_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(outcome, _)| outcome)
                .ok_or_else(|| AppError::BusinessLogic("No votes found".to_string()))?;

            info!("Consensus threshold reached for event {}, settling with outcome: {}", event_id, winning_outcome);

            // Execute settlement
            drop(votes); // Release lock before async call
            self.execute_settlement(&event, &winning_outcome, None).await?;

            Ok(true) // Settlement executed
        } else {
            info!("Consensus vote recorded ({}/{}) for event {}", vote_count, threshold, event_id);
            Ok(false) // Vote recorded, threshold not reached
        }
    }

    /// Execute the actual settlement
    async fn execute_settlement(
        &self,
        event: &Event,
        winning_outcome: &str,
        settler_wallet: Option<&str>,
    ) -> AppResult<String> {
        // Update event status
        self.event_repo
            .update_status(event.id, EventStatus::Resolved)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Call Solana program to settle on-chain
        let event_pubkey = event.solana_pubkey.as_ref()
            .ok_or_else(|| AppError::BusinessLogic("Event not yet created on-chain".to_string()))?;

        // Get the group's Solana pubkey (for now use group_id as placeholder)
        // TODO: Fetch actual group solana_pubkey from database
        let group_pubkey = event.group_id.to_string();

        let tx_signature = match self.solana_client
            .settle_event(event_pubkey, &group_pubkey, winning_outcome)
            .await {
                Ok(s) => s,
                Err(e) => {
                    error!("On-chain settlement failed: {}", e);
                    format!("settlement_offline_{}", chrono::Utc::now().timestamp())
                }
            };
            

        // ----------------------------------------------------
        // PAYOUT LOGIC
        // ----------------------------------------------------
        
        // Find bets
        let bets = self.bet_repo.find_by_event(event.id).await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Calculate total pool and winning shares
        let total_pool: Decimal = bets.iter().map(|b| b.amount_usdc).sum();
        let winning_bets: Vec<_> = bets.iter().filter(|b| b.outcome == winning_outcome).collect();
        let total_winning_shares: Decimal = winning_bets.iter().map(|b| b.shares).sum();
        
        let settler = settler_wallet.unwrap_or("SYSTEM");
        
        // Create settlement record
        let settlement = self.balance_repo.create_settlement(
             event.id,
             winning_outcome,
             total_pool,
             total_winning_shares,
             settler,
             Some(&tx_signature)
        ).await.map_err(AppError::from)?;
        
        // Group bets by user
        let mut user_bets: std::collections::HashMap<uuid::Uuid, Vec<&crate::models::Bet>> = std::collections::HashMap::new();
        for bet in &bets {
            user_bets.entry(bet.user_id).or_default().push(bet);
        }

        for (user_id, user_bet_list) in user_bets {
             let user_winning_bets: Vec<_> = user_bet_list.iter()
                .filter(|b| b.outcome == winning_outcome)
                .collect();
            
             let user_losing_bets: Vec<_> = user_bet_list.iter()
                .filter(|b| b.outcome != winning_outcome)
                .collect();

             // Process winning bets
             if !user_winning_bets.is_empty() {
                 let user_winning_shares: Decimal = user_winning_bets.iter().map(|b| b.shares).sum();
                 let original_bet_amount: Decimal = user_winning_bets.iter().map(|b| b.amount_usdc).sum();
                 
                 // Calculate payout: user_shares / total_winning_shares * total_pool
                 // Handle division by zero edge case
                 let payout = if total_winning_shares > Decimal::ZERO {
                      (user_winning_shares / total_winning_shares) * total_pool
                 } else {
                      original_bet_amount // Refund logic or burn? Fallback to refund for safety
                 };

                 let winnings = payout - original_bet_amount;

                 // Record payout
                 if let Err(e) = self.balance_repo.create_payout(
                      settlement.id,
                      user_id,
                      user_winning_shares,
                      payout
                 ).await {
                      error!("Failed to create payout record for user {}: {:?}", user_id, e);
                 }

                 // Credit winnings
                 if let Err(e) = self.balance_repo.settle_win(
                      user_id,
                      event.group_id,
                      original_bet_amount,
                      winnings,
                      event.id
                 ).await {
                       error!("Failed to credit winnings for user {}: {:?}", user_id, e);
                 }
             }

             // Process losing bets
             for losing_bet in user_losing_bets {
                 if let Err(e) = self.balance_repo.settle_loss(
                       user_id,
                       event.group_id,
                       losing_bet.amount_usdc,
                       event.id
                 ).await {
                       error!("Failed to process loss for user {}: {:?}", user_id, e);
                 }
             }
        }

        // Broadcast settlement notification
        self.ws_server
            .broadcast_event_settled(event.id, winning_outcome.to_string())
            .await;

        info!("Event {} settled with outcome: {} (tx: {})", event.id, winning_outcome, tx_signature);

        Ok(tx_signature)
    }

    /// Verify settler has permission to settle
    async fn verify_settler_permission(
        &self,
        event: &Event,
        settler_wallet: &str,
    ) -> AppResult<bool> {
        // Get user
        let user = self.get_user_by_wallet(settler_wallet).await?;

        // Check if user is admin of the group
        let role = self.group_member_repo
            .find_role(event.group_id, user.id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        Ok(role.map(|r| r == crate::models::MemberRole::Admin).unwrap_or(false))
    }

    /// Determine winning outcome from oracle data
    async fn determine_outcome_from_oracle(
        &self,
        event: &Event,
        oracle_data: &HashMap<String, String>,
    ) -> AppResult<String> {
        // TODO: Implement oracle-specific logic
        // For now, return first outcome as placeholder
        let outcomes = event.outcomes_vec();
        outcomes.first()
            .cloned()
            .ok_or_else(|| AppError::BusinessLogic("No outcomes found".to_string()))
    }

    /// Get user by wallet address
    async fn get_user_by_wallet(&self, wallet: &str) -> AppResult<crate::models::User> {
        
        let user_repo = crate::repositories::UserRepository::new(self.pool.clone());
        
        user_repo
            .find_or_create_by_wallet(wallet)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))
    }
}


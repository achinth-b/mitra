use crate::error::{AppError, AppResult};
use crate::models::Bet;
use crate::repositories::BetRepository;
use crate::solana_client::SolanaClient;
use crate::state_manager::{MerkleProof, StateManager};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Emergency withdrawal service for trustless withdrawals when backend is down
pub struct EmergencyWithdrawalService {
    bet_repo: Arc<BetRepository>,
    state_manager: Arc<StateManager>,
    solana_client: Arc<SolanaClient>,
}

impl EmergencyWithdrawalService {
    /// Create a new emergency withdrawal service
    pub fn new(
        bet_repo: Arc<BetRepository>,
        state_manager: Arc<StateManager>,
        solana_client: Arc<SolanaClient>,
    ) -> Self {
        Self {
            bet_repo,
            state_manager,
            solana_client,
        }
    }

    /// Generate merkle proof for a bet
    /// 
    /// This allows users to withdraw even if backend is down
    pub async fn generate_merkle_proof(
        &self,
        event_id: Uuid,
        bet_id: Uuid,
    ) -> AppResult<MerkleProof> {
        // Get the bet
        let bet = self.bet_repo
            .find_by_id(bet_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?
            .ok_or_else(|| AppError::NotFound(format!("Bet {} not found", bet_id)))?;

        // Verify bet belongs to event
        if bet.event_id != event_id {
            return Err(AppError::Validation("Bet does not belong to event".to_string()));
        }

        // Generate merkle root and proofs
        let (merkle_root, proofs) = self.state_manager
            .generate_merkle_root(event_id)
            .await
            .map_err(|e| AppError::Database(crate::database::DatabaseError::PoolCreation(e)))?;

        // Get proof for this bet
        let proof = proofs.get(&bet_id)
            .ok_or_else(|| AppError::NotFound("Merkle proof not found for bet".to_string()))?
            .clone();

        info!("Generated merkle proof for bet {} in event {}", bet_id, event_id);

        Ok(proof)
    }

    /// Verify merkle proof against on-chain root
    pub async fn verify_proof_against_chain(
        &self,
        event_pubkey: &str,
        proof: &MerkleProof,
    ) -> AppResult<bool> {
        // TODO: Fetch last_merkle_root from Solana EventState account
        // For now, return placeholder
        warn!("Proof verification against chain not yet implemented");
        Ok(true)
    }

    /// Check if emergency withdrawal is available
    /// 
    /// Emergency withdrawal is available if:
    /// - Backend has been down for >24 hours
    /// - Last merkle root was committed >24 hours ago
    pub async fn is_emergency_withdrawal_available(
        &self,
        event_pubkey: &str,
    ) -> AppResult<bool> {
        // TODO: Check last commit time from Solana
        // For now, return false
        Ok(false)
    }

    /// Calculate withdrawal amount for a bet
    /// 
    /// This calculates how much the user can withdraw based on:
    /// - Bet shares
    /// - Current prices (if event not settled)
    /// - Winning outcome (if event settled)
    pub async fn calculate_withdrawal_amount(
        &self,
        bet: &Bet,
        event_settled: bool,
        winning_outcome: Option<&str>,
    ) -> AppResult<Decimal> {
        if event_settled {
            // If event is settled, calculate winnings
            if let Some(winning) = winning_outcome {
                if bet.outcome == winning {
                    // User wins: calculate payout
                    // For LMSR: payout = shares * (1 / final_price)
                    // Simplified: return shares value
                    return Ok(bet.shares);
                } else {
                    // User loses: no withdrawal
                    return Ok(Decimal::ZERO);
                }
            }
        }

        // Event not settled: can withdraw bet amount (minus fees if any)
        // For MVP, allow full withdrawal
        Ok(bet.amount_usdc)
    }
}


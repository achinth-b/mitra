use crate::amm::LmsrAmm;
use crate::auth;
use crate::error::{AppError, AppResult};
use crate::models::{Bet, Transaction, TransactionType, UserGroupBalance};
use crate::repositories::{BalanceRepository, BetRepository, EventRepository, UserRepository};
use crate::services::event_service::EventPrices;
use crate::solana_client::SolanaClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Service for managing bets
pub struct BettingService {
    bet_repo: Arc<BetRepository>,
    event_repo: Arc<EventRepository>,
    user_repo: Arc<UserRepository>,
    balance_repo: Arc<BalanceRepository>,
    solana_client: Arc<SolanaClient>,
}

pub struct BetResult {
    pub bet: Bet,
    pub shares: f64,
    pub price: f64,
    pub updated_prices: EventPrices,
}

impl BettingService {
    pub fn new(
        bet_repo: Arc<BetRepository>,
        event_repo: Arc<EventRepository>,
        user_repo: Arc<UserRepository>,
        balance_repo: Arc<BalanceRepository>,
        solana_client: Arc<SolanaClient>,
    ) -> Self {
        Self {
            bet_repo,
            event_repo,
            user_repo,
            balance_repo,
            solana_client,
        }
    }

    /// Place a bet
    pub async fn place_bet(
        &self,
        event_id: Uuid,
        user_wallet: &str,
        outcome: &str,
        amount_usdc: f64,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<BetResult> {
        info!(
            "Placing bet: event={}, outcome={}, amount={}",
            event_id, outcome, amount_usdc
        );

        // Verify signature
        auth::verify_auth_with_timestamp(user_wallet, "place_bet", timestamp, signature)?;

        // Get Event
        let event = self
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

        // Get User
        let user = self.user_repo.find_or_create_by_wallet(user_wallet).await?;

        // Validate Amount
        let amount_decimal = Decimal::from_f64_retain(amount_usdc).ok_or_else(|| AppError::Validation("Invalid amount".into()))?;

        if amount_decimal <= Decimal::ZERO {
            return Err(AppError::Validation("Amount must be positive".into()));
        }

        // Check Balance
        let balance = self
            .balance_repo
            .get_or_create_balance(user.id, event.group_id)
            .await
            .map_err(AppError::from)?;

        let available = balance.balance_usdc - balance.locked_usdc;
        if available < amount_decimal {
            return Err(AppError::BusinessLogic(format!(
                "Insufficient balance: available {} USDC",
                available
            )));
        }

        // AMM Calculation
        let bets = self
            .bet_repo
            .find_by_event(event_id)
            .await
            .map_err(AppError::from)?;

        let mut amm = LmsrAmm::new(Decimal::new(100, 0), event.outcomes_vec())
            .map_err(|e| AppError::Message(format!("AMM error: {}", e)))?;

        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| AppError::Message(format!("AMM error: {}", e)))?;
        }

        let (shares, price, new_prices) = amm
            .calculate_buy(outcome, amount_decimal)
            .map_err(|e| AppError::Message(format!("AMM calculation error: {}", e)))?;

        // Lock Funds
        self.balance_repo
            .lock_for_bet(user.id, event.group_id, amount_decimal, event_id)
            .await
            .map_err(AppError::from)?;

        // Create Bet
        let bet = self
            .bet_repo
            .create(event_id, user.id, outcome, shares, price, amount_decimal)
            .await
            .map_err(AppError::from)?;

        // Prepare response pricing
        let prices_f64 = new_prices
            .iter()
            .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
            .collect();

        let total_volume: f64 = bets
            .iter()
            .map(|b| b.amount_usdc.to_f64().unwrap_or(0.0))
            .sum::<f64>()
            + amount_usdc;

        Ok(BetResult {
            bet,
            shares: shares.to_f64().unwrap_or(0.0),
            price: price.to_f64().unwrap_or(0.0),
            updated_prices: EventPrices {
                prices: prices_f64,
                total_volume,
            },
        })
    }

    /// Deposit funds
    pub async fn deposit_funds(
        &self,
        group_id: Uuid,
        user_wallet: &str,
        user_usdc_account: &str,
        amount_sol: u64,
        amount_usdc: u64,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<(UserGroupBalance, String)> {
        info!("Deposit funds: user={}, group={}", user_wallet, group_id);

        auth::verify_auth_with_timestamp(user_wallet, "deposit_funds", timestamp, signature)?;

        if amount_sol == 0 && amount_usdc == 0 {
            return Err(AppError::Validation("Must deposit at least some SOL or USDC".into()));
        }

        let from_wallet = Pubkey::from_str(user_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid wallet: {}", e)))?;
        let usdc_account = Pubkey::from_str(user_usdc_account)
            .map_err(|e| AppError::Validation(format!("Invalid USDC account: {}", e)))?;

        let tx_sig = self.solana_client
            .deposit_to_treasury(
                &group_id.to_string(),
                &from_wallet,
                &usdc_account,
                amount_sol,
                amount_usdc,
            )
            .await?;

        let user = self.user_repo.find_or_create_by_wallet(user_wallet).await?;

        // Convert u64 raw amounts to Decimal for DB (assuming 6 decimals for USDC)
        let amount_decimal = Decimal::from(amount_usdc) / Decimal::from(1_000_000); 

        let balance = self.balance_repo
            .credit_balance(
                user.id,
                group_id,
                amount_decimal,
                TransactionType::Deposit,
                None,
                Some(&tx_sig),
                Some("Deposit"),
            )
            .await
            .map_err(AppError::from)?;

        Ok((balance, tx_sig))
    }

    /// Withdraw funds
    pub async fn withdraw_funds(
        &self,
        group_id: Uuid,
        user_wallet: &str,
        user_usdc_account: &str,
        amount_usdc: u64,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<(UserGroupBalance, String)> {
        info!("Withdraw funds: user={}, group={}", user_wallet, group_id);

        auth::verify_auth_with_timestamp(user_wallet, "withdraw_funds", timestamp, signature)?;

        if amount_usdc == 0 {
            return Err(AppError::Validation("Withdraw amount must be positive".into()));
        }

        let user_pubkey = Pubkey::from_str(user_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid wallet: {}", e)))?;

        let user = self.user_repo.find_or_create_by_wallet(user_wallet).await?;

        let amount_decimal = Decimal::from(amount_usdc) / Decimal::from(1_000_000);
        
        let current_balance = self.balance_repo.get_balance(user.id, group_id).await.map_err(AppError::from)?;
        if let Some(b) = current_balance {
            let available = b.balance_usdc - b.locked_usdc;
            if available < amount_decimal {
                 return Err(AppError::BusinessLogic("Insufficient funds".into()));
            }
        } else {
             return Err(AppError::BusinessLogic("No balance found".into()));
        }

        let usdc_pubkey = Pubkey::from_str(user_usdc_account)
            .map_err(|e| AppError::Validation(format!("Invalid USDC account: {}", e)))?;

        let tx_sig = self.solana_client
            .withdraw_from_treasury(
                &group_id.to_string(),
                &user_pubkey,
                &usdc_pubkey,
                0, // amount_sol
                amount_usdc,
            )
            .await?;

        let balance = self.balance_repo
            .debit_balance(
                user.id,
                group_id,
                amount_decimal,
                TransactionType::Withdrawal,
                None,
                Some(&tx_sig),
                Some("Withdrawal"),
            )
            .await
            .map_err(AppError::from)?;

        Ok((balance, tx_sig))
    }

    /// Get Portfolio
    pub async fn get_user_portfolio(
        &self,
        user_wallet: &str,
        group_id: Uuid,
    ) -> AppResult<(UserGroupBalance, Vec<Transaction>)> {
        let user = self.user_repo.find_or_create_by_wallet(user_wallet).await?;
        
        let balance = self.balance_repo
            .get_or_create_balance(user.id, group_id)
            .await
            .map_err(AppError::from)?;

        let transactions = self.balance_repo
            .get_user_transactions(user.id, 50)
            .await
            .map_err(AppError::from)?;

        Ok((balance, transactions))
    }
    
    /// Claim Winnings
    pub async fn claim_winnings(
        &self,
        user_wallet: &str,
        event_id: Uuid,
        user_usdc_account: &str,
        amount_usdc: u64,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<String> {
        // Auth is handled in withdraw_funds too, but we check here or let it propagate?
        // withdraw_funds checks "withdraw_funds" action. claim_winnings checks "claim_winnings" action.
        // We should verify "claim_winnings" signature here.
        auth::verify_auth_with_timestamp(user_wallet, "claim_winnings", timestamp, signature)?;

        // Get Event to find Group ID
        let event = self.event_repo.find_by_id(event_id).await.map_err(AppError::from)?
            .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

        // Delegate to withdraw_funds
        // Note: usage of "withdraw_funds" internal logic would require duplicating signature check or making internal helper.
        // For simplicity, we'll just call logic directly or bypass signature check?
        // Reuse internal logic would be best.
        // But withdraw_funds checks "withdraw_funds" action! User signed "claim_winnings"!
        // So we cannot call `withdraw_funds` public method directly because it will fail auth verification.
        // We must duplicate the logic or extract `withdraw_internal`.
        
        // Extraction is cleaner. But for now, duplicating the simple logic (balance check + solana call + db update) is safer than refactoring large existing method blindly.
        // Actually, logic is:
        // 1. Validate Amount
        // 2. Validate Wallet/USDC Account
        // 3. Check Balance (DB)
        // 4. Solana Withdraw
        // 5. DB Debit

        if amount_usdc == 0 {
            return Err(AppError::Validation("Claim amount must be positive".into()));
        }

        let user_pubkey = Pubkey::from_str(user_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid wallet: {}", e)))?;
        let usdc_pubkey = Pubkey::from_str(user_usdc_account)
            .map_err(|e| AppError::Validation(format!("Invalid USDC account: {}", e)))?;

        let user = self.user_repo.find_or_create_by_wallet(user_wallet).await?;

        let amount_decimal = Decimal::from(amount_usdc) / Decimal::from(1_000_000);
        
        let current_balance = self.balance_repo.get_balance(user.id, event.group_id).await.map_err(AppError::from)?;
        if let Some(b) = current_balance {
            let available = b.balance_usdc - b.locked_usdc;
            if available < amount_decimal {
                 return Err(AppError::BusinessLogic("Insufficient funds to claim".into()));
            }
        } else {
             return Err(AppError::BusinessLogic("No balance found".into()));
        }

        let tx_sig = self.solana_client
            .withdraw_from_treasury(
                &event.group_id.to_string(),
                &user_pubkey,
                &usdc_pubkey,
                0, 
                amount_usdc,
            )
            .await?;

        self.balance_repo
            .debit_balance(
                user.id,
                event.group_id,
                amount_decimal,
                TransactionType::Withdrawal, // Or separate Claim type? Use Withdrawal for now.
                None,
                Some(&tx_sig),
                Some("Claim Winnings"),
            )
            .await
            .map_err(AppError::from)?;

        Ok(tx_sig)
    }
}

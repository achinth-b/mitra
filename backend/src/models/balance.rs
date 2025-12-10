//! Balance and transaction models for fund tracking

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User balance within a specific group
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserGroupBalance {
    pub user_id: Uuid,
    pub group_id: Uuid,
    pub balance_usdc: Decimal,
    pub locked_usdc: Decimal,
    pub updated_at: NaiveDateTime,
}

impl UserGroupBalance {
    /// Get available balance (total - locked)
    pub fn available(&self) -> Decimal {
        self.balance_usdc - self.locked_usdc
    }
}

/// Transaction types for fund movements
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    BetPlaced,
    BetWon,
    BetLost,
    Refund,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdrawal => "withdrawal",
            Self::BetPlaced => "bet_placed",
            Self::BetWon => "bet_won",
            Self::BetLost => "bet_lost",
            Self::Refund => "refund",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "deposit" => Some(Self::Deposit),
            "withdrawal" => Some(Self::Withdrawal),
            "bet_placed" => Some(Self::BetPlaced),
            "bet_won" => Some(Self::BetWon),
            "bet_lost" => Some(Self::BetLost),
            "refund" => Some(Self::Refund),
            _ => None,
        }
    }
}

/// Transaction record for audit trail
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub group_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub transaction_type: String,
    pub amount_usdc: Decimal,
    pub balance_before: Decimal,
    pub balance_after: Decimal,
    pub solana_tx_signature: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

impl Transaction {
    pub fn tx_type(&self) -> Option<TransactionType> {
        TransactionType::from_str(&self.transaction_type)
    }

    pub fn is_confirmed(&self) -> bool {
        self.status == "confirmed"
    }
}

/// Settlement record for an event
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Settlement {
    pub id: Uuid,
    pub event_id: Uuid,
    pub winning_outcome: String,
    pub total_pool: Decimal,
    pub total_winning_shares: Decimal,
    pub settled_by_wallet: String,
    pub solana_tx_signature: Option<String>,
    pub settled_at: NaiveDateTime,
}

/// Individual payout for a user from a settlement
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Payout {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub user_id: Uuid,
    pub shares: Decimal,
    pub payout_amount: Decimal,
    pub claimed: bool,
    pub claimed_at: Option<NaiveDateTime>,
    pub solana_tx_signature: Option<String>,
    pub created_at: NaiveDateTime,
}


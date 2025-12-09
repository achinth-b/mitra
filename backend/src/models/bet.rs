use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Bet model representing an individual bet placed on an event
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bet {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub outcome: String,
    pub shares: Decimal, // DECIMAL(20, 8) in database
    pub price: Decimal,  // DECIMAL(5, 4) in database (0.01 to 0.99)
    pub amount_usdc: Decimal, // DECIMAL(20, 8) in database
    pub timestamp: NaiveDateTime,
    // Note: committed_slot and merkle_proof will be added in Phase 7 migration
    // pub committed_slot: Option<i64>,
    // pub merkle_proof: Option<Value>,
}

impl Bet {
    /// Create a new Bet
    pub fn new(
        event_id: Uuid,
        user_id: Uuid,
        outcome: String,
        shares: Decimal,
        price: Decimal,
        amount_usdc: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_id,
            user_id,
            outcome,
            shares,
            price,
            amount_usdc,
            timestamp: chrono::Utc::now().naive_utc(),
        }
    }

    /// Calculate the total value of the bet (shares * price)
    /// This should equal amount_usdc, but useful for validation
    pub fn total_value(&self) -> Decimal {
        self.shares * self.price
    }

    /// Validate that the bet amounts are consistent
    pub fn validate(&self) -> Result<(), String> {
        if self.shares <= Decimal::ZERO {
            return Err("Shares must be greater than zero".to_string());
        }
        if self.price < Decimal::new(1, 2) || self.price > Decimal::new(99, 2) {
            return Err("Price must be between 0.01 and 0.99".to_string());
        }
        if self.amount_usdc <= Decimal::ZERO {
            return Err("Amount must be greater than zero".to_string());
        }
        Ok(())
    }
}
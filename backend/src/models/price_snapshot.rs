//! Price snapshot model for historical price tracking.
//!
//! This module is deferred for MVP. Uncomment and implement when
//! adding the price history feature.

#![allow(dead_code)]

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Price Snapshot model for historical price tracking.
///
/// Captures a point-in-time snapshot of an outcome's price and liquidity.
/// Used for historical analysis and charting.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PriceSnapshot {
    /// Unique identifier for this snapshot.
    pub id: Uuid,
    /// The event this price snapshot belongs to.
    pub event_id: Uuid,
    /// The outcome name (e.g., "YES", "NO").
    pub outcome: String,
    /// Price at this snapshot (0.01 to 0.99).
    pub price: Decimal,
    /// Liquidity parameter at this snapshot.
    pub liquidity: Decimal,
    /// When this snapshot was taken.
    pub timestamp: NaiveDateTime,
}

impl PriceSnapshot {
    /// Create a new price snapshot.
    pub fn new(
        event_id: Uuid,
        outcome: String,
        price: Decimal,
        liquidity: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_id,
            outcome,
            price,
            liquidity,
            timestamp: chrono::Utc::now().naive_utc(),
        }
    }
}

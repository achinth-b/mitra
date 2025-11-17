use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Event status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Active,
    Resolved,
    Cancelled,
}

impl EventStatus {
    /// Convert from database string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "active" => Ok(EventStatus::Active),
            "resolved" => Ok(EventStatus::Resolved),
            "cancelled" => Ok(EventStatus::Cancelled),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }

    /// Convert to database string
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Active => "active",
            EventStatus::Resolved => "resolved",
            EventStatus::Cancelled => "cancelled",
        }
    }
}

impl From<String> for EventStatus {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or(EventStatus::Active)
    }
}

impl From<EventStatus> for String {
    fn from(status: EventStatus) -> Self {
        status.as_str().to_string()
    }
}

/// Settlement type for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettlementType {
    Manual,
    Oracle,
    Consensus,
}

impl SettlementType {
    /// Convert from database string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "manual" => Ok(SettlementType::Manual),
            "oracle" => Ok(SettlementType::Oracle),
            "consensus" => Ok(SettlementType::Consensus),
            _ => Err(format!("Invalid settlement type: {}", s)),
        }
    }

    /// Convert to database string
    pub fn as_str(&self) -> &'static str {
        match self {
            SettlementType::Manual => "manual",
            SettlementType::Oracle => "oracle",
            SettlementType::Consensus => "consensus",
        }
    }
}

impl From<String> for SettlementType {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or(SettlementType::Manual)
    }
}

impl From<SettlementType> for String {
    fn from(settlement_type: SettlementType) -> Self {
        settlement_type.as_str().to_string()
    }
}

/// Event model representing a prediction market event
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: Uuid,
    pub group_id: Uuid,
    pub solana_pubkey: Option<String>, // Nullable until on-chain creation
    pub title: String,
    pub description: Option<String>, // Nullable for MVP
    pub outcomes: Value, // JSONB stored as serde_json::Value
    pub settlement_type: String, // Stored as TEXT, use SettlementType enum for type safety
    pub status: String, // Stored as TEXT, use EventStatus enum for type safety
    pub resolve_by: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

impl Event {
    /// Create a new Event
    pub fn new(
        group_id: Uuid,
        title: String,
        description: Option<String>,
        outcomes: Vec<String>,
        settlement_type: SettlementType,
        resolve_by: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            group_id,
            solana_pubkey: None,
            title,
            description,
            outcomes: serde_json::to_value(outcomes).unwrap_or(Value::Array(vec![])),
            settlement_type: settlement_type.as_str().to_string(),
            status: EventStatus::Active.as_str().to_string(),
            resolve_by,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    /// Get outcomes as a vector of strings
    pub fn outcomes_vec(&self) -> Vec<String> {
        match &self.outcomes {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        }
    }

    /// Get status as an enum
    pub fn status_enum(&self) -> EventStatus {
        EventStatus::from_str(&self.status).unwrap_or(EventStatus::Active)
    }

    /// Get settlement type as an enum
    pub fn settlement_type_enum(&self) -> SettlementType {
        SettlementType::from_str(&self.settlement_type).unwrap_or(SettlementType::Manual)
    }

    /// Check if event is active
    pub fn is_active(&self) -> bool {
        self.status_enum() == EventStatus::Active
    }

    /// Check if event is resolved
    pub fn is_resolved(&self) -> bool {
        self.status_enum() == EventStatus::Resolved
    }
}
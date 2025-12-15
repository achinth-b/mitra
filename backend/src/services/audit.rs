use crate::error::{AppError, AppResult};
use crate::models::{Bet, Event};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: i64,
    pub event_type: String, // "bet_placed", "event_created", "event_settled", etc.
    pub event_id: Option<Uuid>,
    pub user_wallet: Option<String>,
    pub details: serde_json::Value,
}

/// Audit trail service for logging all important actions
pub struct AuditTrailService {
    #[allow(dead_code)]
    log_file: PathBuf,
    file_handle: Arc<Mutex<std::fs::File>>,
}

impl AuditTrailService {
    /// Create a new audit trail service
    pub fn new(log_directory: PathBuf) -> AppResult<Self> {
        // Ensure directory exists
        std::fs::create_dir_all(&log_directory)
            .map_err(|e| AppError::Message(format!("Failed to create log directory: {}", e)))?;

        // Create log file with date
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let log_file = log_directory.join(format!("audit_{}.log", date));

        // Open file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| AppError::Message(format!("Failed to open audit log file: {}", e)))?;

        info!("Audit trail initialized: {:?}", log_file);

        Ok(Self {
            log_file,
            file_handle: Arc::new(Mutex::new(file)),
        })
    }

    /// Log an audit entry
    pub async fn log(&self, entry: AuditLogEntry) -> AppResult<()> {
        let json = serde_json::to_string(&entry)
            .map_err(|e| AppError::Serialization(e))?;

        let mut file = self.file_handle.lock().await;
        writeln!(file, "{}", json)
            .map_err(|e| AppError::Message(format!("Failed to write audit log: {}", e)))?;

        file.flush()
            .map_err(|e| AppError::Message(format!("Failed to flush audit log: {}", e)))?;

        Ok(())
    }

    /// Log bet placement
    pub async fn log_bet_placed(
        &self,
        bet: &Bet,
        user_wallet: &str,
    ) -> AppResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "bet_placed".to_string(),
            event_id: Some(bet.event_id),
            user_wallet: Some(user_wallet.to_string()),
            details: serde_json::json!({
                "bet_id": bet.id.to_string(),
                "outcome": bet.outcome,
                "shares": bet.shares.to_string(),
                "price": bet.price.to_string(),
                "amount_usdc": bet.amount_usdc.to_string(),
            }),
        };

        self.log(entry).await
    }

    /// Log event creation
    pub async fn log_event_created(
        &self,
        event: &Event,
        creator_wallet: &str,
    ) -> AppResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "event_created".to_string(),
            event_id: Some(event.id),
            user_wallet: Some(creator_wallet.to_string()),
            details: serde_json::json!({
                "group_id": event.group_id.to_string(),
                "title": event.title,
                "outcomes": event.outcomes_vec(),
                "settlement_type": event.settlement_type,
            }),
        };

        self.log(entry).await
    }

    /// Log event settlement
    pub async fn log_event_settled(
        &self,
        event_id: Uuid,
        winning_outcome: &str,
        settler_wallet: &str,
        tx_signature: &str,
    ) -> AppResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "event_settled".to_string(),
            event_id: Some(event_id),
            user_wallet: Some(settler_wallet.to_string()),
            details: serde_json::json!({
                "winning_outcome": winning_outcome,
                "solana_tx": tx_signature,
            }),
        };

        self.log(entry).await
    }

    /// Log merkle root commitment
    pub async fn log_merkle_commitment(
        &self,
        event_id: Uuid,
        merkle_root: &[u8],
        slot: u64,
        tx_signature: &str,
    ) -> AppResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "merkle_committed".to_string(),
            event_id: Some(event_id),
            user_wallet: None,
            details: serde_json::json!({
                "merkle_root": format!("0x{}", hex::encode(merkle_root)),
                "slot": slot,
                "solana_tx": tx_signature,
            }),
        };

        self.log(entry).await
    }

    /// Log emergency withdrawal
    pub async fn log_emergency_withdrawal(
        &self,
        bet_id: Uuid,
        user_wallet: &str,
        amount: Decimal,
        tx_signature: &str,
    ) -> AppResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "emergency_withdrawal".to_string(),
            event_id: None,
            user_wallet: Some(user_wallet.to_string()),
            details: serde_json::json!({
                "bet_id": bet_id.to_string(),
                "amount": amount.to_string(),
                "solana_tx": tx_signature,
            }),
        };

        self.log(entry).await
    }
}


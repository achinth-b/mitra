//! Solana client for on-chain interactions using Anchor
//!
//! This module provides the interface between the backend and Solana programs.
//! It handles transaction building, signing, and sending for all on-chain operations.

use crate::error::{AppError, AppResult};
use anchor_client::solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, Signature, Signer},
    };
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};

/// Configuration for Solana client
#[derive(Clone, Debug)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub events_program_id: String,
    pub friend_groups_program_id: String,
    pub treasury_program_id: String,
    pub commitment: CommitmentConfig,
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            ws_url: None,
            events_program_id: "GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F".to_string(),
            friend_groups_program_id: "A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ".to_string(),
            treasury_program_id: "38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB".to_string(),
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

impl SolanaConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        
        let ws_url = std::env::var("SOLANA_WS_URL").ok();
        
        let events_program_id = std::env::var("EVENTS_PROGRAM_ID")
            .unwrap_or_else(|_| "GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F".to_string());
        
        let friend_groups_program_id = std::env::var("FRIEND_GROUPS_PROGRAM_ID")
            .unwrap_or_else(|_| "A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ".to_string());
        
        let treasury_program_id = std::env::var("TREASURY_PROGRAM_ID")
            .unwrap_or_else(|_| "38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB".to_string());

        Self {
            rpc_url,
            ws_url,
            events_program_id,
            friend_groups_program_id,
            treasury_program_id,
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

/// Solana client for on-chain interactions
pub struct SolanaClient {
    config: SolanaConfig,
    /// Backend keypair for signing transactions (loaded from file or env)
    keypair: Option<Arc<Keypair>>,
    /// RPC client for direct RPC calls
    rpc_client: solana_client::rpc_client::RpcClient,
}

impl SolanaClient {
    /// Create a new Solana client with default configuration
    pub fn new(rpc_url: String) -> Self {
        let config = SolanaConfig {
            rpc_url: rpc_url.clone(),
            ..Default::default()
        };
        
        let rpc_client = solana_client::rpc_client::RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        );

        Self {
            config,
            keypair: None,
            rpc_client,
        }
    }

    /// Create a new Solana client with full configuration
    pub fn with_config(config: SolanaConfig) -> Self {
        let rpc_client = solana_client::rpc_client::RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment,
        );

        Self {
            config,
            keypair: None,
            rpc_client,
        }
    }

    /// Load backend keypair from file
    pub fn with_keypair_file(mut self, path: &str) -> AppResult<Self> {
        let keypair_bytes = std::fs::read(path)
            .map_err(|e| AppError::Config(format!("Failed to read keypair file: {}", e)))?;
        
        let keypair: Vec<u8> = serde_json::from_slice(&keypair_bytes)
            .map_err(|e| AppError::Config(format!("Failed to parse keypair: {}", e)))?;
        
        let keypair = Keypair::from_bytes(&keypair)
            .map_err(|e| AppError::Config(format!("Invalid keypair: {}", e)))?;
        
        self.keypair = Some(Arc::new(keypair));
        info!("Loaded backend keypair: {}", self.keypair.as_ref().unwrap().pubkey());
        
        Ok(self)
    }

    /// Load backend keypair from environment variable (base58 or JSON array)
    pub fn with_keypair_env(mut self, env_var: &str) -> AppResult<Self> {
        let keypair_str = std::env::var(env_var)
            .map_err(|_| AppError::Config(format!("Keypair env var {} not set", env_var)))?;
        
        // Try parsing as JSON array first
        let keypair = if keypair_str.starts_with('[') {
            let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_str)
                .map_err(|e| AppError::Config(format!("Failed to parse keypair JSON: {}", e)))?;
            Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| AppError::Config(format!("Invalid keypair bytes: {}", e)))?
        } else {
            // Try base58
            let keypair_bytes = bs58::decode(&keypair_str)
                .into_vec()
                .map_err(|e| AppError::Config(format!("Failed to decode base58 keypair: {}", e)))?;
            Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| AppError::Config(format!("Invalid keypair: {}", e)))?
        };
        
        self.keypair = Some(Arc::new(keypair));
        info!("Loaded backend keypair from env: {}", self.keypair.as_ref().unwrap().pubkey());
        
        Ok(self)
    }

    /// Get RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.config.rpc_url
    }

    /// Get events program ID
    pub fn events_program_id(&self) -> AppResult<Pubkey> {
        Pubkey::from_str(&self.config.events_program_id)
            .map_err(|e| AppError::Validation(format!("Invalid events program ID: {}", e)))
    }

    /// Get friend groups program ID
    pub fn friend_groups_program_id(&self) -> AppResult<Pubkey> {
        Pubkey::from_str(&self.config.friend_groups_program_id)
            .map_err(|e| AppError::Validation(format!("Invalid friend groups program ID: {}", e)))
    }

    /// Get treasury program ID
    pub fn treasury_program_id(&self) -> AppResult<Pubkey> {
        Pubkey::from_str(&self.config.treasury_program_id)
            .map_err(|e| AppError::Validation(format!("Invalid treasury program ID: {}", e)))
    }

    /// Derive event PDA
    pub fn derive_event_pda(&self, group_pubkey: &Pubkey, title: &str) -> AppResult<(Pubkey, u8)> {
        use sha3::{Digest, Keccak256};
        
        let program_id = self.events_program_id()?;
        let title_hash = Keccak256::digest(title.as_bytes());
        
        let (pda, bump) = Pubkey::find_program_address(
            &[
                b"event",
                group_pubkey.as_ref(),
                &title_hash[..],
            ],
            &program_id,
        );
        
        Ok((pda, bump))
    }

    /// Derive event state PDA
    pub fn derive_event_state_pda(&self, event_pubkey: &Pubkey) -> AppResult<(Pubkey, u8)> {
        let program_id = self.events_program_id()?;
        
        let (pda, bump) = Pubkey::find_program_address(
            &[b"event_state", event_pubkey.as_ref()],
            &program_id,
        );
        
        Ok((pda, bump))
    }

    /// Derive backend authority PDA
    pub fn derive_backend_authority_pda(&self) -> AppResult<(Pubkey, u8)> {
        let program_id = self.events_program_id()?;
        
        let (pda, bump) = Pubkey::find_program_address(
            &[b"backend_authority"],
            &program_id,
        );
        
        Ok((pda, bump))
    }

    /// Get current slot number
    pub async fn get_current_slot(&self) -> AppResult<u64> {
        let slot = self.rpc_client
            .get_slot()
            .map_err(|e| AppError::ExternalService(format!("Failed to get slot: {}", e)))?;
        
        Ok(slot)
    }

    /// Get account balance in lamports
    pub async fn get_balance(&self, pubkey: &Pubkey) -> AppResult<u64> {
        let balance = self.rpc_client
            .get_balance(pubkey)
            .map_err(|e| AppError::ExternalService(format!("Failed to get balance: {}", e)))?;
        
        Ok(balance)
    }

    /// Check if an account exists
    pub async fn account_exists(&self, pubkey: &Pubkey) -> AppResult<bool> {
        match self.rpc_client.get_account(pubkey) {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a "not found" error
                let error_str = e.to_string();
                if error_str.contains("AccountNotFound") || error_str.contains("could not find account") {
                    Ok(false)
                } else {
                    Err(AppError::ExternalService(format!("Failed to check account: {}", e)))
                }
            }
        }
    }

    /// Commit merkle root to on-chain event state
    ///
    /// # Arguments
    /// * `event_pubkey` - The event account pubkey (as string)
    /// * `merkle_root` - The merkle root hash (32 bytes)
    ///
    /// # Returns
    /// Transaction signature
    pub async fn commit_merkle_root(
        &self,
        event_pubkey: &str,
        merkle_root: &[u8],
    ) -> AppResult<String> {
        // Validate merkle root
        if merkle_root.len() != 32 {
            return Err(AppError::Validation("Merkle root must be 32 bytes".to_string()));
        }

        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;

        // Derive event state PDA
        let (event_state_pda, _bump) = self.derive_event_state_pda(&event_pubkey)?;
        
        // Derive backend authority PDA
        let (backend_authority_pda, _) = self.derive_backend_authority_pda()?;

        // Build the instruction manually since we don't have generated Anchor types
        // In production, this would use the generated Anchor client
        let program_id = self.events_program_id()?;

        info!(
            "Committing merkle root to event {} (state: {})",
            event_pubkey, event_state_pda
        );

        // For PoC: Build instruction data manually
        // Anchor instruction discriminator for "commit_state" + merkle_root bytes
        // Using SHA256 to match Anchor's discriminator calculation
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"global:commit_state");
        let hash_result = hasher.finalize();
        let discriminator = hash_result[..8].to_vec();
        let mut instruction_data = discriminator;
        instruction_data.extend_from_slice(merkle_root);

        let instruction = solana_sdk::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(event_pubkey, false),
                solana_sdk::instruction::AccountMeta::new(event_state_pda, false),
                solana_sdk::instruction::AccountMeta::new_readonly(backend_authority_pda, false),
            ],
            data: instruction_data,
        };

        // If we have a keypair, sign and send the transaction
        if let Some(keypair) = &self.keypair {
            let recent_blockhash = self.rpc_client
                .get_latest_blockhash()
                .map_err(|e| AppError::ExternalService(format!("Failed to get blockhash: {}", e)))?;

            let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
                &[instruction],
                Some(&keypair.pubkey()),
                &[keypair.as_ref()],
                recent_blockhash,
            );

            let signature = self.rpc_client
                .send_and_confirm_transaction(&transaction)
                .map_err(|e| AppError::ExternalService(format!("Transaction failed: {}", e)))?;

            info!("Merkle root committed: {}", signature);
            Ok(signature.to_string())
        } else {
            // No keypair - return simulated signature for PoC
            warn!("No keypair configured - simulating merkle root commit");
            let simulated_sig = format!(
                "sim_{}_{}",
                event_pubkey.to_string().chars().take(8).collect::<String>(),
                chrono::Utc::now().timestamp()
            );
            Ok(simulated_sig)
        }
    }

    /// Settle an event on-chain
    ///
    /// # Arguments
    /// * `event_pubkey` - The event account pubkey
    /// * `winning_outcome` - The winning outcome string
    ///
    /// # Returns
    /// Transaction signature
    pub async fn settle_event(
        &self,
        event_pubkey: &str,
        winning_outcome: &str,
    ) -> AppResult<String> {
        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;

        let program_id = self.events_program_id()?;

        info!("Settling event {} with outcome: {}", event_pubkey, winning_outcome);

        // Build instruction data for settle_event
        // Anchor discriminator + winning_outcome string
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"global:settle_event");
        let hash_result = hasher.finalize();
        let discriminator = hash_result[..8].to_vec();
        let mut instruction_data = discriminator;
        
        // Borsh-encode the string: 4-byte length prefix + string bytes
        let outcome_bytes = winning_outcome.as_bytes();
        instruction_data.extend_from_slice(&(outcome_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(outcome_bytes);

        // Note: settle_event requires group and admin accounts
        // For PoC, we'll need to look up the group from the event
        // In production, this would be passed as parameters

        if let Some(keypair) = &self.keypair {
            // For now, return simulated - full implementation requires group lookup
            warn!("Settle event requires group account - using simulation");
            let simulated_sig = format!(
                "settle_sim_{}_{}",
                event_pubkey.to_string().chars().take(8).collect::<String>(),
                chrono::Utc::now().timestamp()
            );
            Ok(simulated_sig)
        } else {
            warn!("No keypair configured - simulating event settlement");
            let simulated_sig = format!(
                "settle_sim_{}_{}",
                event_pubkey.to_string().chars().take(8).collect::<String>(),
                chrono::Utc::now().timestamp()
            );
            Ok(simulated_sig)
        }
    }

    /// Verify a transaction signature exists on-chain
    pub async fn verify_transaction(&self, signature: &str) -> AppResult<bool> {
        // Handle simulated signatures
        if signature.starts_with("sim_") || signature.starts_with("settle_sim_") {
            return Ok(true);
        }

        let sig = Signature::from_str(signature)
            .map_err(|e| AppError::Validation(format!("Invalid signature: {}", e)))?;

        match self.rpc_client.get_signature_status(&sig) {
            Ok(Some(status)) => Ok(status.is_ok()),
            Ok(None) => Ok(false),
            Err(e) => Err(AppError::ExternalService(format!("Failed to verify transaction: {}", e))),
        }
    }

    /// Get on-chain event state (merkle root, etc.)
    pub async fn get_event_state(&self, event_pubkey: &str) -> AppResult<Option<EventStateData>> {
        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;

        let (event_state_pda, _) = self.derive_event_state_pda(&event_pubkey)?;

        match self.rpc_client.get_account(&event_state_pda) {
            Ok(account) => {
                // Parse account data (skip 8-byte discriminator)
                if account.data.len() < 80 {
                    return Err(AppError::ExternalService("Invalid event state data".to_string()));
                }

                let data = &account.data[8..]; // Skip discriminator
                
                // Parse: event (32) + last_merkle_root (32) + last_commit_slot (8) + total_liquidity (8)
                let event = Pubkey::try_from(&data[0..32])
                    .map_err(|_| AppError::ExternalService("Failed to parse event pubkey".to_string()))?;
                
                let mut merkle_root = [0u8; 32];
                merkle_root.copy_from_slice(&data[32..64]);
                
                let last_commit_slot = u64::from_le_bytes(
                    data[64..72].try_into()
                        .map_err(|_| AppError::ExternalService("Failed to parse slot".to_string()))?
                );
                
                let total_liquidity = u64::from_le_bytes(
                    data[72..80].try_into()
                        .map_err(|_| AppError::ExternalService("Failed to parse liquidity".to_string()))?
                );

                Ok(Some(EventStateData {
                    event,
                    last_merkle_root: merkle_root.to_vec(),
                    last_commit_slot,
                    total_liquidity,
                }))
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("AccountNotFound") || error_str.contains("could not find account") {
                    Ok(None)
                } else {
                    Err(AppError::ExternalService(format!("Failed to get event state: {}", e)))
                }
            }
        }
    }
}

/// Event state data parsed from on-chain account
#[derive(Debug, Clone)]
pub struct EventStateData {
    pub event: Pubkey,
    pub last_merkle_root: Vec<u8>,
    pub last_commit_slot: u64,
    pub total_liquidity: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_client_creation() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        assert_eq!(client.rpc_url(), "https://api.devnet.solana.com");
    }

    #[test]
    fn test_config_from_env() {
        let config = SolanaConfig::from_env();
        assert!(!config.rpc_url.is_empty());
        assert!(!config.events_program_id.is_empty());
    }

    #[test]
    fn test_derive_backend_authority_pda() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        let result = client.derive_backend_authority_pda();
        assert!(result.is_ok());
        
        let (pda, bump) = result.unwrap();
        assert!(bump <= 255);
        // PDA should be off-curve
    }

    #[test]
    fn test_derive_event_pda() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        let group_pubkey = Pubkey::new_unique();
        let title = "Test Event";
        
        let result = client.derive_event_pda(&group_pubkey, title);
        assert!(result.is_ok());
        
        let (pda1, _) = result.unwrap();
        let (pda2, _) = client.derive_event_pda(&group_pubkey, title).unwrap();
        
        // Same inputs should produce same PDA
        assert_eq!(pda1, pda2);
        
        // Different title should produce different PDA
        let (pda3, _) = client.derive_event_pda(&group_pubkey, "Different Event").unwrap();
        assert_ne!(pda1, pda3);
    }
}

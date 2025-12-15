//! Solana client for on-chain interactions using Anchor
//!
//! This module provides the interface between the backend and Solana programs.
//! It handles transaction building, signing, and sending for all on-chain operations.

use crate::error::{AppError, AppResult};
use anchor_client::solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use sha2::{Sha256, Digest};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn, debug};
use spl_associated_token_account;
use spl_token;

/// Configuration for Solana client
#[derive(Clone, Debug)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub events_program_id: String,
    pub friend_groups_program_id: String,
    pub treasury_program_id: String,
    pub usdc_mint: String,
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
            usdc_mint: "42ASHzH26iCwtVDhNKHBwWfzn2wt6ikVrXwR8CS3HmjP".to_string(),
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

        let usdc_mint = std::env::var("USDC_MINT")
            .unwrap_or_else(|_| "42ASHzH26iCwtVDhNKHBwWfzn2wt6ikVrXwR8CS3HmjP".to_string());

        Self {
            rpc_url,
            ws_url,
            events_program_id,
            friend_groups_program_id,
            treasury_program_id,
            usdc_mint,
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
        
        let keypair = Keypair::from_bytes(keypair.as_slice())
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
            Keypair::from_bytes(keypair_bytes.as_slice())
                .map_err(|e| AppError::Config(format!("Invalid keypair bytes: {}", e)))?
        } else {
            // Try base58
            let keypair_bytes = bs58::decode(&keypair_str)
                .into_vec()
                .map_err(|e| AppError::Config(format!("Failed to decode base58 keypair: {}", e)))?;
            Keypair::from_bytes(keypair_bytes.as_slice())
                .map_err(|e| AppError::Config(format!("Invalid keypair: {}", e)))?
        };
        
        self.keypair = Some(Arc::new(keypair));
        info!("Loaded backend keypair from env: {}", self.keypair.as_ref().unwrap().pubkey());
        
        Ok(self)
    }

    /// Check if keypair is loaded
    pub fn has_keypair(&self) -> bool {
        self.keypair.is_some()
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

    /// Get USDC mint
    pub fn usdc_mint(&self) -> AppResult<Pubkey> {
        Pubkey::from_str(&self.config.usdc_mint)
            .map_err(|e| AppError::Validation(format!("Invalid USDC mint: {}", e)))
    }

    // ========================================================================
    // PDA Derivation
    // ========================================================================

    /// Derive event PDA from group and title
    pub fn derive_event_pda(&self, group_pubkey: &Pubkey, title: &str) -> AppResult<(Pubkey, u8)> {
        use sha3::{Keccak256, Digest as Sha3Digest};
        
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

    /// Derive friend group PDA
    pub fn derive_friend_group_pda(&self, admin: &Pubkey) -> AppResult<(Pubkey, u8)> {
        let program_id = self.friend_groups_program_id()?;
        
        let (pda, bump) = Pubkey::find_program_address(
            &[b"friend_group", admin.as_ref()],
            &program_id,
        );
        
        Ok((pda, bump))
    }

    // ========================================================================
    // Utility Functions
    // ========================================================================

    /// Calculate Anchor instruction discriminator
    /// Anchor uses first 8 bytes of SHA256("global:<instruction_name>")
    fn instruction_discriminator(name: &str) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(format!("global:{}", name).as_bytes());
        let hash = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash[..8]);
        discriminator
    }

    /// Calculate Anchor account discriminator
    /// Anchor uses first 8 bytes of SHA256("account:<AccountName>")
    fn account_discriminator(name: &str) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(format!("account:{}", name).as_bytes());
        let hash = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash[..8]);
        discriminator
    }

    /// Send and confirm a transaction
    async fn send_transaction(&self, instruction: Instruction) -> AppResult<Signature> {
        let keypair = self.keypair.as_ref()
            .ok_or_else(|| AppError::Config("No keypair configured".to_string()))?;

        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .map_err(|e| AppError::ExternalService(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&keypair.pubkey()),
            &[keypair.as_ref()],
            recent_blockhash,
        );

        let signature = self.rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| AppError::ExternalService(format!("Transaction failed: {}", e)))?;

        Ok(signature)
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
                let error_str = e.to_string();
                if error_str.contains("AccountNotFound") || error_str.contains("could not find account") {
                    Ok(false)
                } else {
                    Err(AppError::ExternalService(format!("Failed to check account: {}", e)))
                }
            }
        }
    }

    // ========================================================================
    // commit_merkle_root - Commits bet state hash to on-chain EventState
    // ========================================================================

    /// Commit merkle root to on-chain event state
    ///
    /// This commits a hash of all off-chain bets to the blockchain,
    /// providing tamper-proof evidence of bet state for emergency withdrawals.
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

        // Check if we have a keypair
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating merkle root commit");
            return Ok(format!(
                "sim_commit_{}_{}",
                &event_pubkey.to_string()[..8],
                chrono::Utc::now().timestamp()
            ));
        }

        // Derive PDAs
        let (event_state_pda, _) = self.derive_event_state_pda(&event_pubkey)?;
        let (backend_authority_pda, _) = self.derive_backend_authority_pda()?;
        let program_id = self.events_program_id()?;

        info!(
            "Committing merkle root to event {} (state PDA: {})",
            event_pubkey, event_state_pda
        );
        debug!("Merkle root: {}", hex::encode(merkle_root));

        // Build instruction data: discriminator (8) + merkle_root (32)
        let discriminator = Self::instruction_discriminator("commit_state");
        let mut instruction_data = Vec::with_capacity(40);
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(merkle_root);

        // Build instruction
        // Accounts: event_contract (mut), event_state (mut), backend_authority
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(event_pubkey, false),
                AccountMeta::new(event_state_pda, false),
                AccountMeta::new_readonly(backend_authority_pda, false),
            ],
            data: instruction_data,
        };

        // Send transaction
        let signature = self.send_transaction(instruction).await?;
        
        info!("Merkle root committed successfully: {}", signature);
        Ok(signature.to_string())
    }

    // ========================================================================
    // settle_event - Settles an event with the winning outcome
    // ========================================================================

    /// Settle an event on-chain
    ///
    /// Marks the event as resolved with the winning outcome.
    /// Only the group admin can settle events.
    ///
    /// # Arguments
    /// * `event_pubkey` - The event account pubkey
    /// * `group_pubkey` - The friend group account pubkey
    /// * `winning_outcome` - The winning outcome string
    ///
    /// # Returns
    /// Transaction signature
    pub async fn settle_event(
        &self,
        event_pubkey: &str,
        group_pubkey: &str,
        winning_outcome: &str,
    ) -> AppResult<String> {
        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;
        
        let group_pubkey = Pubkey::from_str(group_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid group pubkey: {}", e)))?;

        // Check if we have a keypair (admin must sign)
        let keypair = match &self.keypair {
            Some(kp) => kp.clone(),
            None => {
                warn!("No keypair configured - simulating event settlement");
                return Ok(format!(
                    "sim_settle_{}_{}",
                    &event_pubkey.to_string()[..8],
                    chrono::Utc::now().timestamp()
                ));
            }
        };

        let program_id = self.events_program_id()?;

        info!(
            "Settling event {} with outcome: {}",
            event_pubkey, winning_outcome
        );

        // Build instruction data: discriminator (8) + string (4 byte len + bytes)
        let discriminator = Self::instruction_discriminator("settle_event");
        let outcome_bytes = winning_outcome.as_bytes();
        
        let mut instruction_data = Vec::with_capacity(8 + 4 + outcome_bytes.len());
        instruction_data.extend_from_slice(&discriminator);
        // Borsh string encoding: 4-byte little-endian length prefix
        instruction_data.extend_from_slice(&(outcome_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(outcome_bytes);

        // Build instruction
        // Accounts: event_contract (mut), group, admin (signer)
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(event_pubkey, false),
                AccountMeta::new_readonly(group_pubkey, false),
                AccountMeta::new_readonly(keypair.pubkey(), true), // admin signer
            ],
            data: instruction_data,
        };

        // Send transaction
        let signature = self.send_transaction(instruction).await?;
        
        info!("Event settled successfully: {}", signature);
        Ok(signature.to_string())
    }

    /// Settle an event (legacy API - looks up group from event)
    /// 
    /// This version is kept for backwards compatibility but requires
    /// fetching the event first to get the group.
    pub async fn settle_event_legacy(
        &self,
        event_pubkey: &str,
        winning_outcome: &str,
    ) -> AppResult<String> {
        // First, fetch the event to get the group
        let event_data = self.get_event_contract(event_pubkey).await?
            .ok_or_else(|| AppError::NotFound(format!("Event {} not found", event_pubkey)))?;
        
        self.settle_event(event_pubkey, &event_data.group.to_string(), winning_outcome).await
    }

    // ========================================================================
    // get_event_state - Fetches EventState account data
    // ========================================================================

    /// Get on-chain event state (merkle root, liquidity, etc.)
    ///
    /// Returns the current state of an event including the last
    /// committed merkle root and total liquidity.
    pub async fn get_event_state(&self, event_pubkey: &str) -> AppResult<Option<EventStateData>> {
        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;

        let (event_state_pda, _) = self.derive_event_state_pda(&event_pubkey)?;

        debug!("Fetching event state for {} (PDA: {})", event_pubkey, event_state_pda);

        match self.rpc_client.get_account(&event_state_pda) {
            Ok(account) => {
                // EventState layout (after 8-byte discriminator):
                // - event: Pubkey (32 bytes)
                // - last_merkle_root: [u8; 32] (32 bytes)
                // - last_commit_slot: u64 (8 bytes)
                // - total_liquidity: u64 (8 bytes)
                // Total: 8 + 32 + 32 + 8 + 8 = 88 bytes
                
                if account.data.len() < 88 {
                    return Err(AppError::ExternalService(format!(
                        "Invalid event state data: expected 88 bytes, got {}",
                        account.data.len()
                    )));
                }

                // Verify discriminator
                let expected_discriminator = Self::account_discriminator("EventState");
                if account.data[..8] != expected_discriminator {
                    return Err(AppError::ExternalService(
                        "Invalid EventState discriminator".to_string()
                    ));
                }

                let data = &account.data[8..]; // Skip discriminator
                
                // Parse fields
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

                debug!(
                    "Event state: slot={}, liquidity={}, merkle_root={}",
                    last_commit_slot,
                    total_liquidity,
                    hex::encode(&merkle_root[..8])
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
                    debug!("Event state not found for {}", event_pubkey);
                    Ok(None)
                } else {
                    Err(AppError::ExternalService(format!("Failed to get event state: {}", e)))
                }
            }
        }
    }

    // ========================================================================
    // get_event_contract - Fetches EventContract account data
    // ========================================================================

    /// Get on-chain event contract data
    pub async fn get_event_contract(&self, event_pubkey: &str) -> AppResult<Option<EventContractData>> {
        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;

        match self.rpc_client.get_account(&event_pubkey) {
            Ok(account) => {
                if account.data.len() < 80 {
                    return Err(AppError::ExternalService("Invalid event contract data".to_string()));
                }

                // Verify discriminator
                let expected_discriminator = Self::account_discriminator("EventContract");
                if account.data[..8] != expected_discriminator {
                    return Err(AppError::ExternalService(
                        "Invalid EventContract discriminator".to_string()
                    ));
                }

                let data = &account.data[8..]; // Skip discriminator
                
                // Parse event_id and group (first 64 bytes)
                let event_id = Pubkey::try_from(&data[0..32])
                    .map_err(|_| AppError::ExternalService("Failed to parse event_id".to_string()))?;
                
                let group = Pubkey::try_from(&data[32..64])
                    .map_err(|_| AppError::ExternalService("Failed to parse group".to_string()))?;

                // Title is next (4 byte len + string)
                // For now, we just need group - full parsing can be added later
                
                Ok(Some(EventContractData {
                    event_id,
                    group,
                }))
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("AccountNotFound") || error_str.contains("could not find account") {
                    Ok(None)
                } else {
                    Err(AppError::ExternalService(format!("Failed to get event contract: {}", e)))
                }
            }
        }
    }

    // ========================================================================
    // create_friend_group - Creates a new friend group on-chain
    // ========================================================================

    /// Create a new friend group on-chain
    ///
    /// Initializes the friend group PDA and treasury accounts.
    ///
    /// # Arguments
    /// * `name` - Group name
    /// * `admin_wallet` - Admin wallet pubkey
    ///
    /// # Returns
    /// (Transaction signature, Group Pubkey)
    pub async fn create_friend_group(
        &self,
        name: &str,
        admin_wallet: &str,
    ) -> AppResult<(String, String)> {
        let admin_pubkey = Pubkey::from_str(admin_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid admin wallet: {}", e)))?;
        
        let groups_program_id = self.friend_groups_program_id()?;
        
        // Derive Group PDA
        let (group_pda, _) = self.derive_friend_group_pda(&admin_pubkey)?;
        
        // Check if exists
        if let Ok(true) = self.account_exists(&group_pda).await {
             // If simulated, might always return false, so this check is good for real env
             info!("Group already exists on-chain: {}", group_pda);
             // In a real scenario we might want to fail or return existing. 
             // For now, proceed to attempt creation or return keys.
        }

        // Check keypair
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating group creation");
            return Ok((
                format!("sim_create_group_{}", chrono::Utc::now().timestamp()),
                group_pda.to_string()
            ));
        }

        let usdc_mint = self.usdc_mint()?;

        // Derive Treasury PDAs (SOL and USDC)
        let (treasury_sol_pda, _) = Pubkey::find_program_address(
            &[b"treasury_sol", group_pda.as_ref()],
            &groups_program_id,
        );

        // For USDC treasury (token account), it's usually an ATA owned by the Group PDA
        // The program usually initializes this ATA.
        // Or if it's a PDA based token account:
        let (treasury_usdc_pda, _) = Pubkey::find_program_address(
            &[b"treasury_usdc", group_pda.as_ref()],
            &groups_program_id,
        );
        // Note: Logic depends on program. If program uses ATA, we calculate ATA.
        // Assuming program uses PDA seeds for token account based on `deposit_to_treasury` not calculating ATA.

        info!("Creating group '{}' for admin {} (PDA: {})", name, admin_wallet, group_pda);

        // Build instruction data: discriminator (8) + name string
        let discriminator = Self::instruction_discriminator("create_group");
        let name_bytes = name.as_bytes();
        let mut instruction_data = Vec::with_capacity(8 + 4 + name_bytes.len());
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(name_bytes);

        let system_program = Pubkey::from_str("11111111111111111111111111111111")
            .map_err(|_| AppError::Config("Invalid system program ID".to_string()))?;
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|_| AppError::Config("Invalid token program ID".to_string()))?;
        let rent = Pubkey::from_str("SysvarRent111111111111111111111111111111111")
            .map_err(|_| AppError::Config("Invalid rent sysvar ID".to_string()))?;

        // Accounts: group, admin, usdc_mint, treasury_sol, treasury_usdc, payer (signer), system, token, rent
        let payer = self.keypair.as_ref().unwrap().pubkey();

        let instruction = Instruction {
            program_id: groups_program_id,
            accounts: vec![
                AccountMeta::new(group_pda, false),              // group
                AccountMeta::new_readonly(admin_pubkey, false),  // admin (not signing, payer signs)
                AccountMeta::new_readonly(usdc_mint, false),     // usdc_mint
                AccountMeta::new(treasury_sol_pda, false),       // treasury_sol
                AccountMeta::new(treasury_usdc_pda, false),      // treasury_usdc
                AccountMeta::new(payer, true),                   // payer (signer)
                AccountMeta::new_readonly(system_program, false),// system_program
                AccountMeta::new_readonly(token_program, false), // token_program
                AccountMeta::new_readonly(rent, false),          // rent
            ],
            data: instruction_data,
        };

        let signature = self.send_transaction(instruction).await?;
        
        info!("Group created successfully on-chain: {}", signature);
        Ok((signature.to_string(), group_pda.to_string()))
    }


    /// Deposit funds (SOL and/or USDC) to a friend group treasury
    ///
    /// The user must be a member of the group. Funds are tracked per-member
    /// in the GroupMember account on-chain.
    ///
    /// # Arguments
    /// * `group_pubkey` - The friend group account pubkey
    /// * `user_wallet` - The user's wallet pubkey (must sign)
    /// * `user_usdc_account` - The user's USDC token account
    /// * `amount_sol` - Amount of SOL to deposit (in lamports)
    /// * `amount_usdc` - Amount of USDC to deposit (in smallest units)
    ///
    /// # Returns
    /// Transaction signature
    pub async fn deposit_to_treasury(
        &self,
        group_pubkey: &str,
        user_wallet: &Pubkey,
        user_usdc_account: &Pubkey,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> AppResult<String> {
        if amount_sol == 0 && amount_usdc == 0 {
            return Err(AppError::Validation("Must deposit at least some SOL or USDC".to_string()));
        }

        let group_pubkey = Pubkey::from_str(group_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid group pubkey: {}", e)))?;

        // Check if we have a keypair
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating deposit");
            return Ok(format!(
                "sim_deposit_{}_{}",
                &group_pubkey.to_string()[..8],
                chrono::Utc::now().timestamp()
            ));
        }

        let program_id = self.friend_groups_program_id()?;

        // Derive PDAs
        let (member_pda, _) = Pubkey::find_program_address(
            &[b"member", group_pubkey.as_ref(), user_wallet.as_ref()],
            &program_id,
        );

        let (treasury_sol_pda, _) = Pubkey::find_program_address(
            &[b"treasury_sol", group_pubkey.as_ref()],
            &program_id,
        );

        // Get treasury_usdc from group account (would need to fetch, using placeholder)
        // In production, fetch the group account to get treasury_usdc address
        let treasury_usdc = self.get_group_treasury_usdc(&group_pubkey).await?;

        info!(
            "Depositing to group {}: {} lamports SOL, {} USDC",
            group_pubkey, amount_sol, amount_usdc
        );

        // Build instruction data: discriminator (8) + amount_sol (8) + amount_usdc (8)
        let discriminator = Self::instruction_discriminator("deposit_funds");
        let mut instruction_data = Vec::with_capacity(24);
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount_sol.to_le_bytes());
        instruction_data.extend_from_slice(&amount_usdc.to_le_bytes());

        // Token program ID
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|_| AppError::Config("Invalid token program ID".to_string()))?;
        let system_program = Pubkey::from_str("11111111111111111111111111111111")
            .map_err(|_| AppError::Config("Invalid system program ID".to_string()))?;

        // Build instruction
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(group_pubkey, false),           // friend_group
                AccountMeta::new(member_pda, false),             // member
                AccountMeta::new(treasury_sol_pda, false),       // treasury_sol
                AccountMeta::new(treasury_usdc, false),          // treasury_usdc
                AccountMeta::new(*user_usdc_account, false),     // member_usdc_account
                AccountMeta::new(*user_wallet, true),            // member_wallet (signer)
                AccountMeta::new_readonly(token_program, false), // token_program
                AccountMeta::new_readonly(system_program, false),// system_program
            ],
            data: instruction_data,
        };

        let signature = self.send_transaction(instruction).await?;
        
        info!("Deposit successful: {}", signature);
        Ok(signature.to_string())
    }

    // ========================================================================
    // withdraw_from_treasury - Withdraws funds from group treasury
    // ========================================================================

    /// Withdraw funds (SOL and/or USDC) from a friend group treasury
    ///
    /// The user must be a member with sufficient balance. Funds cannot be
    /// withdrawn if they are locked (e.g., active bets).
    ///
    /// # Arguments
    /// * `group_pubkey` - The friend group account pubkey
    /// * `user_wallet` - The user's wallet pubkey (must sign)
    /// * `user_usdc_account` - The user's USDC token account
    /// * `amount_sol` - Amount of SOL to withdraw (in lamports)
    /// * `amount_usdc` - Amount of USDC to withdraw (in smallest units)
    ///
    /// # Returns
    /// Transaction signature
    pub async fn withdraw_from_treasury(
        &self,
        group_pubkey: &str,
        user_wallet: &Pubkey,
        user_usdc_account: &Pubkey,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> AppResult<String> {
        if amount_sol == 0 && amount_usdc == 0 {
            return Err(AppError::Validation("Must withdraw at least some SOL or USDC".to_string()));
        }

        let group_pubkey = Pubkey::from_str(group_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid group pubkey: {}", e)))?;

        // Check if we have a keypair
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating withdrawal");
            return Ok(format!(
                "sim_withdraw_{}_{}",
                &group_pubkey.to_string()[..8],
                chrono::Utc::now().timestamp()
            ));
        }

        let program_id = self.friend_groups_program_id()?;

        // Derive PDAs
        let (member_pda, _) = Pubkey::find_program_address(
            &[b"member", group_pubkey.as_ref(), user_wallet.as_ref()],
            &program_id,
        );

        let (treasury_sol_pda, _) = Pubkey::find_program_address(
            &[b"treasury_sol", group_pubkey.as_ref()],
            &program_id,
        );

        let treasury_usdc = self.get_group_treasury_usdc(&group_pubkey).await?;

        info!(
            "Withdrawing from group {}: {} lamports SOL, {} USDC",
            group_pubkey, amount_sol, amount_usdc
        );

        // Build instruction data
        let discriminator = Self::instruction_discriminator("withdraw_funds");
        let mut instruction_data = Vec::with_capacity(24);
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount_sol.to_le_bytes());
        instruction_data.extend_from_slice(&amount_usdc.to_le_bytes());

        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|_| AppError::Config("Invalid token program ID".to_string()))?;
        let system_program = Pubkey::from_str("11111111111111111111111111111111")
            .map_err(|_| AppError::Config("Invalid system program ID".to_string()))?;

        // Build instruction
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(group_pubkey, false),           // friend_group
                AccountMeta::new(member_pda, false),             // member
                AccountMeta::new(treasury_sol_pda, false),       // treasury_sol
                AccountMeta::new(treasury_usdc, false),          // treasury_usdc
                AccountMeta::new(*user_usdc_account, false),     // member_usdc_account
                AccountMeta::new(*user_wallet, true),            // member_wallet (signer)
                AccountMeta::new_readonly(token_program, false), // token_program
                AccountMeta::new_readonly(system_program, false),// system_program
            ],
            data: instruction_data,
        };

        let signature = self.send_transaction(instruction).await?;
        
        info!("Withdrawal successful: {}", signature);
        Ok(signature.to_string())
    }

    // ========================================================================
    // claim_winnings - Claims winnings from a resolved event
    // ========================================================================

    /// Claim winnings from a resolved event
    ///
    /// After an event is settled, winners can claim their USDC winnings.
    /// The amount is calculated based on their shares in the winning outcome.
    ///
    /// # Arguments
    /// * `event_pubkey` - The event account pubkey
    /// * `group_pubkey` - The friend group account pubkey
    /// * `user_wallet` - The user's wallet pubkey (must sign)
    /// * `user_usdc_account` - The user's USDC token account
    /// * `amount` - Amount of USDC to claim (in smallest units)
    ///
    /// # Returns
    /// Transaction signature
    pub async fn claim_winnings(
        &self,
        event_pubkey: &str,
        group_pubkey: &str,
        user_wallet: &Pubkey,
        user_usdc_account: &Pubkey,
        amount: u64,
    ) -> AppResult<String> {
        if amount == 0 {
            return Err(AppError::Validation("Claim amount must be positive".to_string()));
        }

        let event_pubkey = Pubkey::from_str(event_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid event pubkey: {}", e)))?;
        
        let group_pubkey = Pubkey::from_str(group_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid group pubkey: {}", e)))?;

        // Check if we have a keypair
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating claim");
            return Ok(format!(
                "sim_claim_{}_{}",
                &event_pubkey.to_string()[..8],
                chrono::Utc::now().timestamp()
            ));
        }

        let events_program_id = self.events_program_id()?;
        let groups_program_id = self.friend_groups_program_id()?;

        // Derive member PDA
        let (member_pda, _) = Pubkey::find_program_address(
            &[b"member", group_pubkey.as_ref(), user_wallet.as_ref()],
            &groups_program_id,
        );

        let treasury_usdc = self.get_group_treasury_usdc(&group_pubkey).await?;

        info!(
            "Claiming {} USDC from event {} for user {}",
            amount, event_pubkey, user_wallet
        );

        // Build instruction data: discriminator (8) + amount (8)
        let discriminator = Self::instruction_discriminator("claim_winnings");
        let mut instruction_data = Vec::with_capacity(16);
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|_| AppError::Config("Invalid token program ID".to_string()))?;

        // Build instruction
        // Accounts: event_contract, group, treasury_usdc, user_usdc_account, member, user (signer), token_program
        let instruction = Instruction {
            program_id: events_program_id,
            accounts: vec![
                AccountMeta::new(event_pubkey, false),           // event_contract
                AccountMeta::new_readonly(group_pubkey, false),  // group
                AccountMeta::new(treasury_usdc, false),          // treasury_usdc
                AccountMeta::new(*user_usdc_account, false),     // user_usdc_account
                AccountMeta::new_readonly(member_pda, false),    // member
                AccountMeta::new(*user_wallet, true),            // user (signer)
                AccountMeta::new_readonly(token_program, false), // token_program
            ],
            data: instruction_data,
        };

        let signature = self.send_transaction(instruction).await?;
        
        info!("Claim successful: {}", signature);
        Ok(signature.to_string())
    }

    // ========================================================================
    // Helper: Get group treasury USDC account
    // ========================================================================

    /// Get the treasury USDC token account for a group
    async fn get_group_treasury_usdc(&self, group_pubkey: &Pubkey) -> AppResult<Pubkey> {
        // Fetch the FriendGroup account to get treasury_usdc
        match self.rpc_client.get_account(group_pubkey) {
            Ok(account) => {
                // FriendGroup layout (after 8-byte discriminator):
                // - admin: Pubkey (32)
                // - name: String (4 + up to 50)
                // - member_count: u32 (4)
                // - treasury_sol: Pubkey (32)
                // - treasury_usdc: Pubkey (32)
                // ...
                
                if account.data.len() < 8 + 32 + 4 + 50 + 4 + 32 + 32 {
                    return Err(AppError::ExternalService("Invalid FriendGroup account data".to_string()));
                }

                let data = &account.data[8..]; // Skip discriminator
                
                // Skip admin (32) + name (variable) + member_count (4) + treasury_sol (32)
                // Name is Borsh-encoded: 4 byte length + string bytes
                let name_len = u32::from_le_bytes(
                    data[32..36].try_into()
                        .map_err(|_| AppError::ExternalService("Failed to parse name length".to_string()))?
                ) as usize;
                
                let treasury_usdc_offset = 32 + 4 + name_len + 4 + 32;
                
                if data.len() < treasury_usdc_offset + 32 {
                    return Err(AppError::ExternalService("FriendGroup data too short for treasury_usdc".to_string()));
                }

                let treasury_usdc = Pubkey::try_from(&data[treasury_usdc_offset..treasury_usdc_offset + 32])
                    .map_err(|_| AppError::ExternalService("Failed to parse treasury_usdc".to_string()))?;

                Ok(treasury_usdc)
            }
            Err(e) => {
                Err(AppError::ExternalService(format!("Failed to get group account: {}", e)))
            }
        }
    }

    /// Get member balance from on-chain account
    pub async fn get_member_balance(
        &self,
        group_pubkey: &str,
        user_wallet: &str,
    ) -> AppResult<Option<MemberBalance>> {
        let group_pubkey = Pubkey::from_str(group_pubkey)
            .map_err(|e| AppError::Validation(format!("Invalid group pubkey: {}", e)))?;
        let user_wallet = Pubkey::from_str(user_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid user wallet: {}", e)))?;

        let program_id = self.friend_groups_program_id()?;

        let (member_pda, _) = Pubkey::find_program_address(
            &[b"member", group_pubkey.as_ref(), user_wallet.as_ref()],
            &program_id,
        );

        match self.rpc_client.get_account(&member_pda) {
            Ok(account) => {
                // GroupMember layout (after 8-byte discriminator):
                // - user: Pubkey (32)
                // - group: Pubkey (32)
                // - role: enum (1)
                // - balance_sol: u64 (8)
                // - balance_usdc: u64 (8)
                // - locked_funds: bool (1)
                // - joined_at: i64 (8)
                
                if account.data.len() < 8 + 32 + 32 + 1 + 8 + 8 + 1 + 8 {
                    return Err(AppError::ExternalService("Invalid GroupMember data".to_string()));
                }

                let data = &account.data[8..]; // Skip discriminator
                
                let balance_sol = u64::from_le_bytes(
                    data[65..73].try_into()
                        .map_err(|_| AppError::ExternalService("Failed to parse balance_sol".to_string()))?
                );
                
                let balance_usdc = u64::from_le_bytes(
                    data[73..81].try_into()
                        .map_err(|_| AppError::ExternalService("Failed to parse balance_usdc".to_string()))?
                );

                let locked_funds = data[81] != 0;

                Ok(Some(MemberBalance {
                    balance_sol,
                    balance_usdc,
                    locked_funds,
                }))
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("AccountNotFound") || error_str.contains("could not find account") {
                    Ok(None)
                } else {
                    Err(AppError::ExternalService(format!("Failed to get member account: {}", e)))
                }
            }
        }
    }

    /// Verify a transaction signature exists on-chain
    pub async fn verify_transaction(&self, signature: &str) -> AppResult<bool> {
        // Handle simulated signatures
        if signature.starts_with("sim_") {
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

    // ========================================================================
    // Faucet
    // ========================================================================

    /// Mint test tokens to a user wallet (Faucet)
    pub async fn mint_test_tokens(
        &self,
        to_wallet: &str,
        amount: u64
    ) -> AppResult<String> {
        let to_pubkey = Pubkey::from_str(to_wallet)
            .map_err(|e| AppError::Validation(format!("Invalid wallet: {}", e)))?;
            
        // Check keypair (Mint Authority)
        if self.keypair.is_none() {
            warn!("No keypair configured - simulating faucet mint");
            return Ok(format!("sim_mint_{}_{}", to_wallet, chrono::Utc::now().timestamp()));
        }
        let payer = self.keypair.as_ref().unwrap();

        let usdc_mint = self.usdc_mint()?;

        // Get ATA
        let ata = spl_associated_token_account::get_associated_token_address(
            &to_pubkey,
            &usdc_mint,
        );

        let mut instructions = vec![];

        // 1. Create ATA if needed (idempotent)
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &payer.pubkey(),
                &to_pubkey,
                &usdc_mint,
                &spl_token::ID,
            )
        );

        // 2. Mint tokens
        instructions.push(
            spl_token::instruction::mint_to(
                &spl_token::ID,
                &usdc_mint,
                &ata,
                &payer.pubkey(),
                &[], // multi-signers
                amount,
            ).map_err(|e| AppError::ExternalService(format!("Failed to build mint instruction: {}", e)))?
        );

        // Send transaction
        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .map_err(|e| AppError::ExternalService(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer.as_ref()], // payer signs as Payer AND Mint Authority
            recent_blockhash,
        );

        let signature = self.rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| AppError::ExternalService(format!("Faucet transaction failed: {}", e)))?;

        info!("Faucet mint successful: {}", signature);
        Ok(signature.to_string())
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// Event state data parsed from on-chain account
#[derive(Debug, Clone)]
pub struct EventStateData {
    pub event: Pubkey,
    pub last_merkle_root: Vec<u8>,
    pub last_commit_slot: u64,
    pub total_liquidity: u64,
}

/// Event contract data parsed from on-chain account
#[derive(Debug, Clone)]
pub struct EventContractData {
    pub event_id: Pubkey,
    pub group: Pubkey,
}

/// Member balance data parsed from on-chain account
#[derive(Debug, Clone)]
pub struct MemberBalance {
    pub balance_sol: u64,
    pub balance_usdc: u64,
    pub locked_funds: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_client_creation() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        assert_eq!(client.rpc_url(), "https://api.devnet.solana.com");
        assert!(!client.has_keypair());
    }

    #[test]
    fn test_config_from_env() {
        let config = SolanaConfig::from_env();
        assert!(!config.rpc_url.is_empty());
        assert!(!config.events_program_id.is_empty());
    }

    #[test]
    fn test_instruction_discriminator() {
        // Test that discriminator is calculated correctly
        let disc = SolanaClient::instruction_discriminator("commit_state");
        assert_eq!(disc.len(), 8);
        
        // Same name should produce same discriminator
        let disc2 = SolanaClient::instruction_discriminator("commit_state");
        assert_eq!(disc, disc2);
        
        // Different name should produce different discriminator
        let disc3 = SolanaClient::instruction_discriminator("settle_event");
        assert_ne!(disc, disc3);
    }

    #[test]
    fn test_account_discriminator() {
        let disc = SolanaClient::account_discriminator("EventState");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_derive_backend_authority_pda() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        let result = client.derive_backend_authority_pda();
        assert!(result.is_ok());
        
        let (pda, bump) = result.unwrap();
        assert!(bump <= 255);
        
        // Same seeds should produce same PDA
        let (pda2, _) = client.derive_backend_authority_pda().unwrap();
        assert_eq!(pda, pda2);
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

    #[test]
    fn test_derive_event_state_pda() {
        let client = SolanaClient::new("https://api.devnet.solana.com".to_string());
        let event_pubkey = Pubkey::new_unique();
        
        let result = client.derive_event_state_pda(&event_pubkey);
        assert!(result.is_ok());
        
        let (pda, _) = result.unwrap();
        
        // Verify it's deterministic
        let (pda2, _) = client.derive_event_state_pda(&event_pubkey).unwrap();
        assert_eq!(pda, pda2);
    }
}

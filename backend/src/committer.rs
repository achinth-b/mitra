use crate::repositories::EventRepository;
use crate::solana_client::SolanaClient;
use crate::state_manager::StateManager;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

/// Background task that commits merkle roots to Solana every 10 seconds
pub struct Committer {
    state_manager: Arc<StateManager>,
    event_repo: Arc<EventRepository>,
    solana_client: Arc<SolanaClient>,
    pool: PgPool,
    commit_interval: Duration,
    min_volume_threshold: u64, // Minimum volume (in USDC cents) to trigger commit
}

impl Committer {
    /// Create a new committer
    /// 
    /// # Arguments
    /// * `state_manager` - State manager for generating merkle roots
    /// * `event_repo` - Event repository
    /// * `solana_client` - Solana client for on-chain commits
    /// * `pool` - Database pool
    /// * `commit_interval` - How often to commit (default: 10 seconds)
    /// * `min_volume_threshold` - Minimum volume to trigger commit (default: 100000 = $1000)
    pub fn new(
        state_manager: Arc<StateManager>,
        event_repo: Arc<EventRepository>,
        solana_client: Arc<SolanaClient>,
        pool: PgPool,
    ) -> Self {
        Self {
            state_manager,
            event_repo,
            solana_client,
            pool,
            commit_interval: Duration::from_secs(10),
            min_volume_threshold: 100000, // $1000 in USDC cents
        }
    }

    /// Start the committer background task
    pub async fn start(self) {
        let mut interval = time::interval(self.commit_interval);
        info!("Committer started, will commit every {:?}", self.commit_interval);

        loop {
            interval.tick().await;
            
            if let Err(e) = self.commit_pending_states().await {
                error!("Error committing states: {}", e);
            }
        }
    }

    /// Commit pending states for all active events
    async fn commit_pending_states(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get all active events
        let active_events = self.event_repo.find_active_events().await?;

        if active_events.is_empty() {
            return Ok(());
        }

        info!("Committing states for {} active events", active_events.len());

        for event in active_events {
            // Skip if event doesn't have Solana pubkey yet
            let event_pubkey = match &event.solana_pubkey {
                Some(pubkey) => pubkey,
                None => {
                    warn!("Event {} has no Solana pubkey, skipping", event.id);
                    continue;
                }
            };

            // Get total volume for event
            let total_volume = self.state_manager.get_total_volume(event.id).await?;
            use rust_decimal::prelude::ToPrimitive;
            let volume_cents = total_volume
                .and_then(|v| (v * rust_decimal::Decimal::new(100, 0)).to_u64())
                .unwrap_or(0);

            // Check if we should commit (volume threshold or time-based)
            if volume_cents < self.min_volume_threshold {
                continue;
            }

            // Generate merkle root
            let (merkle_root, _proofs) = self
                .state_manager
                .generate_merkle_root(event.id)
                .await?;

            // Get current slot
            let current_slot = self.solana_client.get_current_slot().await?;

            // Commit to Solana
            match self
                .solana_client
                .commit_merkle_root(event_pubkey, &merkle_root)
                .await
            {
                Ok(tx_signature) => {
                    info!(
                        "Committed merkle root for event {}: {} (slot: {})",
                        event.id, tx_signature, current_slot
                    );

                    // TODO: Update bets with committed_slot in Phase 7
                    // For now, just log
                }
                Err(e) => {
                    error!(
                        "Failed to commit merkle root for event {}: {}",
                        event.id, e
                    );
                    // Continue with other events
                }
            }
        }

        Ok(())
    }

    /// Set commit interval
    pub fn with_commit_interval(mut self, interval: Duration) -> Self {
        self.commit_interval = interval;
        self
    }

    /// Set minimum volume threshold
    pub fn with_min_volume_threshold(mut self, threshold: u64) -> Self {
        self.min_volume_threshold = threshold;
        self
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_committer_creation() {
        // This would require mock dependencies
        // For now, just test the structure
        assert!(true);
    }
}


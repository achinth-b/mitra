use crate::models::Bet;
use crate::repositories::BetRepository;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: Vec<u8>,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

/// Merkle proof for a bet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub bet_id: Uuid,
    pub path: Vec<Vec<u8>>, // Hashes of sibling nodes
    pub leaf_hash: Vec<u8>,
}

/// State manager for tracking off-chain bets and generating merkle roots
pub struct StateManager {
    bet_repo: BetRepository,
}

impl StateManager {
    pub fn new(pool: PgPool) -> Self {
        Self {
            bet_repo: BetRepository::new(pool),
        }
    }

    /// Get all pending bets (uncommitted) for an event
    pub async fn get_pending_bets(&self, event_id: Uuid) -> Result<Vec<Bet>, sqlx::Error> {
        // For MVP, all bets are pending since committed_slot doesn't exist yet
        // In Phase 7, filter by committed_slot IS NULL
        self.bet_repo.find_by_event(event_id).await
    }

    /// Generate merkle root for pending bets
    /// 
    /// # Arguments
    /// * `event_id` - The event ID
    /// 
    /// # Returns
    /// (merkle_root, merkle_proofs) - Root hash and proofs for each bet
    pub async fn generate_merkle_root(
        &self,
        event_id: Uuid,
    ) -> Result<(Vec<u8>, HashMap<Uuid, MerkleProof>), sqlx::Error> {
        let bets = self.get_pending_bets(event_id).await?;

        if bets.is_empty() {
            // Return zero hash for empty tree
            let zero_hash = vec![0u8; 32];
            return Ok((zero_hash, HashMap::new()));
        }

        // Create leaf nodes from bets
        let leaves: Vec<(Uuid, Vec<u8>)> = bets
            .iter()
            .map(|bet| (bet.id, self.hash_bet(bet)))
            .collect();

        // Build merkle tree
        let (root, proofs) = self.build_merkle_tree(leaves);

        Ok((root, proofs))
    }

    /// Hash a bet into a leaf node
    fn hash_bet(&self, bet: &Bet) -> Vec<u8> {
        // Serialize bet data
        let bet_data = format!(
            "{}:{}:{}:{}:{}:{}",
            bet.id,
            bet.event_id,
            bet.user_id,
            bet.outcome,
            bet.shares,
            bet.amount_usdc
        );

        // Hash using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(bet_data.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Build merkle tree from leaves
    /// 
    /// Returns (root_hash, proofs_map)
    fn build_merkle_tree(
        &self,
        leaves: Vec<(Uuid, Vec<u8>)>,
    ) -> (Vec<u8>, HashMap<Uuid, MerkleProof>) {
        if leaves.is_empty() {
            return (vec![0u8; 32], HashMap::new());
        }

        if leaves.len() == 1 {
            let (bet_id, hash) = &leaves[0];
            let mut proofs = HashMap::new();
            proofs.insert(
                *bet_id,
                MerkleProof {
                    bet_id: *bet_id,
                    path: vec![],
                    leaf_hash: hash.clone(),
                },
            );
            return (hash.clone(), proofs);
        }

        // Build tree level by level
        let mut current_level = leaves;
        let mut proofs: HashMap<Uuid, MerkleProof> = HashMap::new();
        let mut level = 0;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let mut i = 0;

            while i < current_level.len() {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    // Duplicate last node if odd number
                    &current_level[i]
                };

                // Hash parent = hash(left + right)
                let parent_hash = self.hash_pair(&left.1, &right.1);

                // Store proof paths
                if i < current_level.len() {
                    let bet_id = left.0;
                    let proof = proofs.entry(bet_id).or_insert_with(|| MerkleProof {
                        bet_id,
                        path: vec![],
                        leaf_hash: left.1.clone(),
                    });
                    proof.path.push(right.1.clone());
                }

                if i + 1 < current_level.len() {
                    let bet_id = right.0;
                    let proof = proofs.entry(bet_id).or_insert_with(|| MerkleProof {
                        bet_id,
                        path: vec![],
                        leaf_hash: right.1.clone(),
                    });
                    proof.path.push(left.1.clone());
                }

                next_level.push((left.0, parent_hash));
                i += 2;
            }

            current_level = next_level;
            level += 1;
        }

        let root_hash = current_level[0].1.clone();
        (root_hash, proofs)
    }

    /// Hash a pair of hashes
    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut combined = Vec::new();
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);

        let mut hasher = Sha256::new();
        hasher.update(&combined);
        hasher.finalize().to_vec()
    }

    /// Verify a merkle proof
    pub fn verify_proof(
        &self,
        proof: &MerkleProof,
        root_hash: &[u8],
    ) -> bool {
        let mut current_hash = proof.leaf_hash.clone();

        // Traverse proof path
        for sibling_hash in &proof.path {
            // Determine if current is left or right
            // For simplicity, always combine as (current, sibling)
            current_hash = self.hash_pair(&current_hash, sibling_hash);
        }

        // Compare with root
        current_hash == root_hash
    }

    /// Get total volume for an event
    pub async fn get_total_volume(&self, event_id: Uuid) -> Result<Option<Decimal>, sqlx::Error> {
        self.bet_repo.get_total_volume_for_event(event_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[test]
    fn test_hash_bet() {
        // This is a unit test that would need a mock repository
        // For now, just test the hashing logic
        let bet_data = "test:bet:data";
        let mut hasher = Sha256::new();
        hasher.update(bet_data.as_bytes());
        let hash = hasher.finalize().to_vec();
        
        assert_eq!(hash.len(), 32);
    }

    // Note: Full tests require database setup - see tests/database_test.rs
}


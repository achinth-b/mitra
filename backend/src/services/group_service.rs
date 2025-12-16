use crate::auth;
use crate::error::{AppError, AppResult};
use crate::models::{FriendGroup, MemberRole};
use crate::repositories::{FriendGroupRepository, GroupMemberRepository, UserRepository};
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use std::sync::Arc;
use tracing::{info, warn};
use crate::solana_client::SolanaClient;
use uuid::Uuid;

/// Service for managing friend groups
pub struct GroupService {
    group_repo: Arc<FriendGroupRepository>,
    user_repo: Arc<UserRepository>,
    member_repo: Arc<GroupMemberRepository>,
    solana_client: Arc<SolanaClient>,
}

impl GroupService {
    pub fn new(
        group_repo: Arc<FriendGroupRepository>,
        user_repo: Arc<UserRepository>,
        member_repo: Arc<GroupMemberRepository>,
        solana_client: Arc<SolanaClient>,
    ) -> Self {
        Self {
            group_repo,
            user_repo,
            member_repo,
            solana_client,
        }
    }

    /// Create a new friend group
    pub async fn create_group(
        &self,
        name: &str,
        admin_wallet: &str,
        solana_pubkey: Option<&str>,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<FriendGroup> {
        info!("Creating group: name={}, admin={}", name, admin_wallet);

        // Verify signature
        auth::verify_auth_with_timestamp(admin_wallet, "create_group", timestamp, signature)?;

        // Ensure user exists
        let user = self.user_repo.find_or_create_by_wallet(admin_wallet).await?;

        // Generate pubkey if missing (mock/dev mode fallback)
        // Generate pubkey if missing (mock/dev mode fallback)
        let group_pubkey = match solana_pubkey {
            Some(pk) => pk.to_string(),
            None => {
                // Determine if we should create on-chain
                if self.solana_client.has_keypair() {
                    info!("Attempting to create group on-chain...");
                    match self.solana_client.create_friend_group(name, admin_wallet).await {
                        Ok((sig, pubkey)) => {
                            info!("On-chain group creation successful: {}", sig);
                            pubkey
                        },
                        Err(e) => {
                            warn!("Failed to create group on-chain: {}. Falling back to off-chain keypair.", e);
                            // Fallback (for offline dev or errors)
                            // Ideally we should fail here if strict consistency is needed
                            Keypair::new().pubkey().to_string()
                        }
                    }
                } else {
                    info!("No backend keypair, skipping on-chain creation");
                    Keypair::new().pubkey().to_string()
                }
            },
        };

        // Create group - note: repo signature is (solana_pubkey, name, admin_wallet)
        let group = self
            .group_repo
            .create(&group_pubkey, name, admin_wallet)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        // Add admin as first member
        self.member_repo
            .add_member(group.id, user.id, MemberRole::Admin)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        info!("Created group {} ({})", group.name, group.id);
        Ok(group)
    }

    /// Invite a member to a group
    pub async fn invite_member(
        &self,
        group_id: Uuid,
        invited_wallet: &str,
        inviter_wallet: &str,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<(crate::models::User, crate::models::GroupMember)> {
        // Verify signature
        auth::verify_auth_with_timestamp(inviter_wallet, "invite_member", timestamp, signature)?;

        // Verify inviter is a member
        let inviter = self.user_repo.find_or_create_by_wallet(inviter_wallet).await?;
        if !self
            .member_repo
            .is_member(group_id, inviter.id)
            .await
            .map_err(|e| AppError::Database(e.into()))?
        {
            return Err(AppError::Unauthorized("Only members can invite others".into()));
        }

        // Find/Create invited user
        let invited_user = self.user_repo.find_or_create_by_wallet(invited_wallet).await?;

        // Add to group
        let member = self
            .member_repo
            .add_member(group_id, invited_user.id, MemberRole::Member)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        info!("Added member {} to group {}", invited_user.id, group_id);
        Ok((invited_user, member))
    }

    /// Delete a group
    pub async fn delete_group(
        &self,
        group_id: Uuid,
        admin_wallet: &str,
        signature: &str,
        timestamp: i64,
    ) -> AppResult<bool> {
        // Verify signature
        auth::verify_auth_with_timestamp(admin_wallet, "delete_group", timestamp, signature)?;

        // Fetch group
        let group = self
            .group_repo
            .find_by_id(group_id)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .ok_or_else(|| AppError::NotFound("Group not found".into()))?;

        // Verify admin
        if group.admin_wallet != admin_wallet {
            return Err(AppError::Unauthorized("Only group admin can delete".into()));
        }

        // Delete matches
        let success = self.group_repo.delete(group.id).await.map_err(|e| AppError::Database(e.into()))?;
        
        info!("Deleted group {}", group_id);
        Ok(success)
    }
}

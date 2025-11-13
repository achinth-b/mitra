use anchor_lang::prelude::*;

#[account]
pub struct FriendGroup {
    pub admin: Pubkey,              // 32 bytes
    pub name: String,               // 4 + len (max 50 chars = 50 bytes)
    pub member_count: u32,          // 4 bytes
    pub treasury_sol: Pubkey,       // 32 bytes (PDA for SOL)
    pub treasury_usdc: Pubkey,     // 32 bytes (Associated Token Account for USDC)
    pub treasury_bump: u8,          // 1 byte (for PDA derivation)
    pub created_at: i64,            // 8 bytes
}

impl FriendGroup {
    // Calculate space needed for account
    // 8 (discriminator) + sizes above
    pub const MAX_SIZE: usize = 8 + 32 + (4 + 50) + 4 + 32 + 32 + 1 + 8;
    
    pub const MIN_MEMBERS: u32 = 3;
    pub const MAX_MEMBERS: u32 = 30;
}

#[account]
pub struct GroupMember {
    pub user: Pubkey,               // 32 bytes
    pub group: Pubkey,              // 32 bytes
    pub role: MemberRole,           // 1 byte (enum)
    pub balance_sol: u64,           // 8 bytes (available SOL balance)
    pub balance_usdc: u64,          // 8 bytes (available USDC balance)
    pub locked_funds: bool,         // 1 byte (true if removed with active bets)
    pub joined_at: i64,            // 8 bytes
}

impl GroupMember {
    pub const MAX_SIZE: usize = 8 + 32 + 32 + 1 + 8 + 8 + 1 + 8;
}

#[account]
pub struct Invite {
    pub group: Pubkey,              // 32 bytes
    pub invited_user: Pubkey,       // 32 bytes
    pub inviter: Pubkey,            // 32 bytes (who sent the invite)
    pub created_at: i64,           // 8 bytes
    pub expires_at: i64,           // 8 bytes (7 days from created_at)
}

impl Invite {
    pub const MAX_SIZE: usize = 8 + 32 + 32 + 32 + 8 + 8;
    
    pub const EXPIRY_SECONDS: i64 = 7 * 24 * 60 * 60; // 7 days
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum MemberRole {
    Admin,
    Member,
}
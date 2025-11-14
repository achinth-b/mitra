use anchor_lang::prelude::*;

#[account]
pub struct EventContract {
    pub event_id: Pubkey,              // 32 bytes (PDA)
    pub group: Pubkey,                  // 32 bytes (friend group PDA)
    pub title: String,                  // 4 + len (max 100 chars)
    pub description: String,            // 4 + len (max 500 chars)
    pub outcomes: Vec<String>,          // 4 + (4 + len) * count (max 10 outcomes, 50 chars each)
    pub settlement_type: SettlementType, // 1 byte
    pub status: EventStatus,            // 1 byte
    pub resolve_by: i64,               // 8 bytes
    pub total_volume: u64,             // 8 bytes
    pub created_at: i64,               // 8 bytes
    pub settled_at: Option<i64>,       // 1 + 8 bytes (optional)
    pub winning_outcome: Option<String>, // 1 + 4 + len (optional)
}

impl EventContract {
    pub const MAX_SIZE: usize = 8 + // discriminator
        32 + // event_id
        32 + // group
        (4 + 100) + // title
        (4 + 500) + // description
        4 + (4 + 50) * 10 + // outcomes (max 10, 50 chars each)
        1 + // settlement_type
        1 + // status
        8 + // resolve_by
        8 + // total_volume
        8 + // created_at
        1 + 8 + // settled_at (Option<i64>)
        1 + 4 + 50; // winning_outcome (Option<String>, max 50 chars)
}

#[account]
pub struct EventState {
    pub event: Pubkey,                  // 32 bytes
    pub last_merkle_root: [u8; 32],    // 32 bytes
    pub last_commit_slot: u64,         // 8 bytes
    pub total_liquidity: u64,           // 8 bytes
}

impl EventState {
    pub const MAX_SIZE: usize = 8 + // discriminator
        32 + // event
        32 + // last_merkle_root
        8 + // last_commit_slot
        8; // total_liquidity
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum SettlementType {
    Manual,      // Admin decides
    Oracle,      // External oracle (Pyth/Switchboard)
    Consensus,   // Group voting
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum EventStatus {
    Active,      // Accepting bets
    Resolved,    // Settled with winner
    Cancelled,   // Cancelled before resolution
}
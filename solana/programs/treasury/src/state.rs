use anchor_lang::prelude::*;

// Account size constants for clarity
pub const DISCRIMINATOR_SIZE: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U64_SIZE: usize = 8;
pub const U32_SIZE: usize = 4;
pub const U8_SIZE: usize = 1;
pub const I64_SIZE: usize = 8;
pub const OPTION_I64_SIZE: usize = 1 + 8; // Option<i64> = 1 byte tag + 8 bytes data
pub const OPTION_STRING_SIZE: usize = 1 + 4 + 50; // Option<String> = 1 byte tag + 4 bytes len + max 50 chars
pub const STRING_PREFIX_SIZE: usize = 4; // String prefix for length
pub const VEC_PREFIX_SIZE: usize = 4; // Vec prefix for length

/// Settlement record for a single bet
#[account]
pub struct Settlement {
    pub event: Pubkey,                  // 32 bytes - Event PDA
    pub friend_group: Pubkey,           // 32 bytes - Friend group PDA
    pub user: Pubkey,                   // 32 bytes - User who placed the bet
    pub outcome: String,                // 4 + len (max 50 chars) - Winning outcome
    pub amount_won: u64,               // 8 bytes - Amount won in lamports/tokens
    pub token_type: TokenType,         // 1 byte - SOL or USDC
    pub settled_at: i64,               // 8 bytes - Timestamp when settled
    pub settlement_id: u64,            // 8 bytes - Unique settlement ID
}

impl Settlement {
    pub const MAX_SIZE: usize = DISCRIMINATOR_SIZE +
        PUBKEY_SIZE + // event
        PUBKEY_SIZE + // friend_group
        PUBKEY_SIZE + // user
        (STRING_PREFIX_SIZE + 50) + // outcome (max 50 chars)
        U64_SIZE + // amount_won
        U8_SIZE + // token_type
        I64_SIZE + // settled_at
        U64_SIZE; // settlement_id
}

/// Batch settlement record for atomic processing
#[account]
pub struct BatchSettlement {
    pub batch_id: u64,                  // 8 bytes - Unique batch ID
    pub friend_group: Pubkey,           // 32 bytes - Friend group PDA
    pub settlements: Vec<SettlementEntry>, // 4 + (SettlementEntry size) * count
    pub total_sol_amount: u64,         // 8 bytes - Total SOL to distribute
    pub total_usdc_amount: u64,        // 8 bytes - Total USDC to distribute
    pub created_at: i64,               // 8 bytes
    pub executed_at: Option<i64>,      // 1 + 8 bytes - When batch was executed
    pub status: BatchStatus,           // 1 byte
}

impl BatchSettlement {
    pub const MAX_SIZE: usize = DISCRIMINATOR_SIZE +
        U64_SIZE + // batch_id
        PUBKEY_SIZE + // friend_group
        VEC_PREFIX_SIZE + (PUBKEY_SIZE + PUBKEY_SIZE + U64_SIZE + U8_SIZE) * 100 + // settlements (max 100 per batch)
        U64_SIZE + // total_sol_amount
        U64_SIZE + // total_usdc_amount
        I64_SIZE + // created_at
        OPTION_I64_SIZE + // executed_at
        U8_SIZE; // status
    
    pub const MAX_SETTLEMENTS_PER_BATCH: usize = 100;
}

/// Compact settlement entry for batch processing
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SettlementEntry {
    pub user: Pubkey,                   // 32 bytes
    pub event: Pubkey,                  // 32 bytes
    pub amount: u64,                   // 8 bytes
    pub token_type: TokenType,         // 1 byte
}

/// Emergency withdrawal request with timelock
#[account]
pub struct EmergencyWithdraw {
    pub request_id: u64,               // 8 bytes - Unique request ID
    pub friend_group: Pubkey,           // 32 bytes - Friend group PDA
    pub admin: Pubkey,                  // 32 bytes - Admin who requested
    pub destination: Pubkey,            // 32 bytes - Where to send funds
    pub sol_amount: u64,               // 8 bytes - SOL amount to withdraw
    pub usdc_amount: u64,              // 8 bytes - USDC amount to withdraw
    pub requested_at: i64,             // 8 bytes - When request was created
    pub unlock_at: i64,                // 8 bytes - When withdrawal can be executed
    pub executed_at: Option<i64>,       // 1 + 8 bytes - When withdrawal was executed
    pub status: WithdrawStatus,        // 1 byte
}

impl EmergencyWithdraw {
    pub const MAX_SIZE: usize = DISCRIMINATOR_SIZE +
        U64_SIZE + // request_id
        PUBKEY_SIZE + // friend_group
        PUBKEY_SIZE + // admin
        PUBKEY_SIZE + // destination
        U64_SIZE + // sol_amount
        U64_SIZE + // usdc_amount
        I64_SIZE + // requested_at
        I64_SIZE + // unlock_at
        OPTION_I64_SIZE + // executed_at
        U8_SIZE; // status
    
    pub const TIMELOCK_SECONDS: i64 = 7 * 24 * 60 * 60; // 7 days
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum TokenType {
    Sol,
    Usdc,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,    // Created but not executed
    Executed,   // Successfully executed
    Failed,     // Execution failed
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum WithdrawStatus {
    Pending,    // Request created, waiting for timelock
    Executed,   // Successfully executed
    Cancelled,  // Cancelled before execution
}


use anchor_lang::prelude::*;

#[error_code]
pub enum TreasuryError {
    #[msg("Unauthorized: Only admin can perform this action")]
    Unauthorized,
    
    #[msg("Invalid settlement entry")]
    InvalidSettlement,
    
    #[msg("Batch settlement not found")]
    BatchNotFound,
    
    #[msg("Batch already executed")]
    BatchAlreadyExecuted,
    
    #[msg("Too many settlements in batch (max 100)")]
    TooManySettlements,
    
    #[msg("Emergency withdrawal not found")]
    WithdrawNotFound,
    
    #[msg("Emergency withdrawal timelock not expired")]
    TimelockNotExpired,
    
    #[msg("Emergency withdrawal already executed")]
    WithdrawAlreadyExecuted,
    
    #[msg("Emergency withdrawal cancelled")]
    WithdrawCancelled,
    
    #[msg("Invalid friend group")]
    InvalidFriendGroup,
    
    #[msg("Invalid event")]
    InvalidEvent,
    
    #[msg("Insufficient treasury balance")]
    InsufficientBalance,
    
    #[msg("Settlement amount must be greater than zero")]
    InvalidAmount,
    
    #[msg("Event not resolved")]
    EventNotResolved,
    
    #[msg("Invalid token type")]
    InvalidTokenType,
    
    #[msg("Batch settlement failed")]
    BatchExecutionFailed,
    
    #[msg("Invalid treasury account")]
    InvalidTreasury,
    
    #[msg("Invalid destination account")]
    InvalidDestination,
    
    #[msg("Invalid token account")]
    InvalidTokenAccount,
}


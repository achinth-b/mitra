use anchor_lang::prelude::*;

#[error_code]
pub enum EventError {
    #[msg("Unauthorized: Only admin can perform this action")]
    Unauthorized,
    
    #[msg("Event not found")]
    EventNotFound,
    
    #[msg("Event already settled")]
    EventAlreadySettled,
    
    #[msg("Event is cancelled")]
    EventCancelled,
    
    #[msg("Event not yet settled")]
    EventNotSettled,
    
    #[msg("Invalid outcome")]
    InvalidOutcome,
    
    #[msg("Title too long (max 100 characters)")]
    TitleTooLong,
    
    #[msg("Description too long (max 500 characters)")]
    DescriptionTooLong,
    
    #[msg("Too many outcomes (max 10)")]
    TooManyOutcomes,
    
    #[msg("Invalid resolve_by timestamp")]
    InvalidResolveBy,
    
    #[msg("Insufficient winnings or treasury balance")]
    InsufficientWinnings,
    
    #[msg("Winnings already claimed")]
    WinningsAlreadyClaimed,
    
    #[msg("Only backend authority can commit state")]
    NotBackendAuthority,
    
    #[msg("Invalid treasury account")]
    InvalidTreasury,
    
    #[msg("Invalid token mint")]
    InvalidMint,
    
    #[msg("User is not a group member")]
    NotGroupMember,
    
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
}

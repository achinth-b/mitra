use anchor_lang::prelude::*;

#[error_code]
pub enum FriendGroupError {
    #[msg("Only admin can perform this action")]
    Unauthorized,
    
    #[msg("Group name too long (max 50 characters)")]
    NameTooLong,
    
    #[msg("Member already exists in group")]
    MemberAlreadyExists,
    
    #[msg("Member not found in group")]
    MemberNotFound,
    
    #[msg("Insufficient balance")]
    InsufficientBalance,
    
    #[msg("Invalid amount")]
    InvalidAmount,
    
    #[msg("Group has reached maximum member limit")]
    MaxMembersReached,
    
    #[msg("Cannot remove member: group must have at least 3 members")]
    MinMembersRequired,
    
    #[msg("Invite not found or expired")]
    InviteInvalid,
    
    #[msg("Invite has expired")]
    InviteExpired,
    
    #[msg("Member has locked funds from active bets")]
    FundsLocked,
    
    #[msg("Invalid treasury account")]
    InvalidTreasury,
}
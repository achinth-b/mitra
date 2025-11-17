use anchor_lang::prelude::*;
use crate::errors::*;
use crate::state::{BatchSettlement, EmergencyWithdraw, BatchStatus, WithdrawStatus};

pub fn close_batch_settlement(ctx: Context<CloseBatchSettlement>) -> Result<()> {
    let batch = &ctx.accounts.batch_settlement;
    
    require!(
        batch.status == BatchStatus::Executed,
        TreasuryError::BatchAlreadyExecuted
    );
    
    // Account will be closed by Anchor's close constraint
    // Rent will be refunded to admin automatically
    Ok(())
}

pub fn close_emergency_withdraw(ctx: Context<CloseEmergencyWithdraw>) -> Result<()> {
    let withdraw = &ctx.accounts.emergency_withdraw;
    
    require!(
        withdraw.status == WithdrawStatus::Executed,
        TreasuryError::WithdrawAlreadyExecuted
    );
    
    // Account will be closed by Anchor's close constraint
    // Rent will be refunded to admin automatically
    Ok(())
}

#[derive(Accounts)]
pub struct CloseBatchSettlement<'info> {
    #[account(
        mut,
        close = admin, // Close account and send rent to admin
        seeds = [b"batch_settlement", friend_group.key().as_ref(), batch_settlement.batch_id.to_le_bytes().as_ref()],
        bump
    )]
    pub batch_settlement: Account<'info, BatchSettlement>,
    
    #[account(mut)]
    pub friend_group: Account<'info, friend_groups::state::FriendGroup>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(request_id: u64)]
pub struct CloseEmergencyWithdraw<'info> {
    #[account(
        mut,
        close = admin, // Close account and send rent to admin
        seeds = [b"emergency_withdraw", friend_group.key().as_ref(), request_id.to_le_bytes().as_ref()],
        bump
    )]
    pub emergency_withdraw: Account<'info, EmergencyWithdraw>,
    
    #[account(mut)]
    pub friend_group: Account<'info, friend_groups::state::FriendGroup>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
}


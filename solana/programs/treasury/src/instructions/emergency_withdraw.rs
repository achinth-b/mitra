use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;
use crate::state::{EmergencyWithdraw as EmergencyWithdrawAccount, WithdrawStatus};
use friend_groups::state::FriendGroup;

#[derive(Accounts)]
#[instruction(request_id: u64, sol_amount: u64, usdc_amount: u64)]
pub struct EmergencyWithdrawAccounts<'info> {
    #[account(
        init_if_needed,
        payer = admin,
        space = EmergencyWithdrawAccount::MAX_SIZE,
        seeds = [b"emergency_withdraw", friend_group.key().as_ref(), request_id.to_le_bytes().as_ref()],
        bump
    )]
    pub emergency_withdraw: Account<'info, EmergencyWithdrawAccount>,
    
    #[account(
        mut,
        constraint = friend_group.treasury_sol == treasury_sol.key() @ TreasuryError::InvalidFriendGroup,
        constraint = friend_group.treasury_usdc == treasury_usdc.key() @ TreasuryError::InvalidFriendGroup
    )]
    pub friend_group: Account<'info, FriendGroup>,
    
    /// CHECK: SOL treasury PDA (validated by seeds)
    #[account(
        mut,
        seeds = [b"treasury_sol", friend_group.key().as_ref()],
        bump = friend_group.treasury_bump
    )]
    pub treasury_sol: UncheckedAccount<'info>,
    
    /// CHECK: USDC treasury token account
    #[account(
        mut,
        constraint = treasury_usdc.owner == friend_group.key() @ TreasuryError::InvalidTreasury
    )]
    pub treasury_usdc: Account<'info, anchor_spl::token::TokenAccount>,
    
    /// CHECK: Destination wallet for SOL (must not be executable)
    #[account(
        mut,
        constraint = !destination.executable @ TreasuryError::InvalidDestination
    )]
    pub destination: UncheckedAccount<'info>,
    
    /// CHECK: Destination token account for USDC
    #[account(
        mut,
        constraint = destination_token_account.mint == treasury_usdc.mint @ TreasuryError::InvalidDestination,
        constraint = destination_token_account.owner == token_program.key() @ TreasuryError::InvalidTokenAccount
    )]
    pub destination_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<crate::treasury::EmergencyWithdrawAccounts>,
    request_id: u64,
    sol_amount: u64,
    usdc_amount: u64,
) -> Result<()> {
    let withdraw = &mut ctx.accounts.emergency_withdraw;
    let friend_group = &ctx.accounts.friend_group;
    let clock = Clock::get()?;
    
    // Validate admin
    require!(
        friend_group.admin == ctx.accounts.admin.key(),
        TreasuryError::Unauthorized
    );
    
    // Validate amounts
    require!(
        sol_amount > 0 || usdc_amount > 0,
        TreasuryError::InvalidAmount
    );
    
    // Check if this is an existing request that can be executed
    if withdraw.request_id == request_id && withdraw.status == WithdrawStatus::Pending {
        // Execute withdrawal - timelock must have expired
        require!(
            withdraw.unlock_at <= clock.unix_timestamp,
            TreasuryError::TimelockNotExpired
        );
        
        require!(
            withdraw.status == WithdrawStatus::Pending,
            TreasuryError::WithdrawAlreadyExecuted
        );
        
        // Reentrancy protection: ensure we're not already executing
        require!(
            withdraw.executed_at.is_none(),
            TreasuryError::WithdrawAlreadyExecuted
        );
        
        // Validate treasury balances
        let treasury_sol_balance = ctx.accounts.treasury_sol.lamports();
        let treasury_usdc_balance = ctx.accounts.treasury_usdc.amount;
        
        require!(
            treasury_sol_balance >= withdraw.sol_amount,
            TreasuryError::InsufficientBalance
        );
        
        require!(
            treasury_usdc_balance >= withdraw.usdc_amount,
            TreasuryError::InsufficientBalance
        );
        
        // Transfer SOL
        if withdraw.sol_amount > 0 {
            **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= withdraw.sol_amount;
            **ctx.accounts.destination.to_account_info().try_borrow_mut_lamports()? += withdraw.sol_amount;
        }
        
        // Transfer USDC
        if withdraw.usdc_amount > 0 {
            let friend_group_account_info = ctx.accounts.friend_group.to_account_info();
            let seeds = &[
                b"friend_group",
                friend_group.admin.as_ref(),
                &[ctx.accounts.friend_group.friend_group_bump],
            ];
            let signer_seeds = &[&seeds[..]];
            
            let cpi_accounts = Transfer {
                from: ctx.accounts.treasury_usdc.to_account_info(),
                to: ctx.accounts.destination_token_account.to_account_info(),
                authority: friend_group_account_info,
            };
            
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            
            token::transfer(cpi_ctx, withdraw.usdc_amount)?;
        }
        
        // Update status
        withdraw.status = WithdrawStatus::Executed;
        withdraw.executed_at = Some(clock.unix_timestamp);
    } else {
        // Create new request - ensure account is uninitialized or status allows new request
        require!(
            withdraw.request_id == 0 || withdraw.status != WithdrawStatus::Pending,
            TreasuryError::WithdrawAlreadyExecuted
        );
        
        // Prevent overwriting an executed request with same ID
        if withdraw.request_id == request_id && withdraw.status == WithdrawStatus::Executed {
            return Err(TreasuryError::WithdrawAlreadyExecuted.into());
        }
        
        // Set up new request
        withdraw.request_id = request_id;
        withdraw.friend_group = ctx.accounts.friend_group.key();
        withdraw.admin = ctx.accounts.admin.key();
        withdraw.destination = ctx.accounts.destination.key();
        withdraw.sol_amount = sol_amount;
        withdraw.usdc_amount = usdc_amount;
        withdraw.requested_at = clock.unix_timestamp;
        withdraw.unlock_at = clock.unix_timestamp
            .checked_add(EmergencyWithdrawAccount::TIMELOCK_SECONDS)
            .ok_or(TreasuryError::InvalidAmount)?;
        withdraw.status = WithdrawStatus::Pending;
        withdraw.executed_at = None;
    }
    
    Ok(())
}


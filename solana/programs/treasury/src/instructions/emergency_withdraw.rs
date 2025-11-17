use anchor_lang::prelude::*;
use crate::errors::*;
use crate::state::{EmergencyWithdraw as EmergencyWithdrawAccount, WithdrawStatus};

pub fn handler(
    ctx: Context<crate::treasury::EmergencyWithdrawAccounts>,
    request_id: u64,
    sol_amount: u64,
    usdc_amount: u64,
) -> Result<()> {
    let withdraw = &mut ctx.accounts.emergency_withdraw;
    let friend_group = &ctx.accounts.friend_group;
    let clock = Clock::get()?;
    
    // Validate treasury_sol matches friend_group
    require!(
        ctx.accounts.treasury_sol.key() == friend_group.treasury_sol,
        TreasuryError::InvalidFriendGroup
    );
    
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
        
        // Transfer SOL and/or USDC via CPI to friend_groups program
        if withdraw.sol_amount > 0 || withdraw.usdc_amount > 0 {
            let cpi_program = ctx.accounts.friend_groups_program.to_account_info();
            let cpi_accounts = friend_groups::cpi::accounts::TreasuryTransfer {
                friend_group: ctx.accounts.friend_group.to_account_info(),
                treasury_sol: ctx.accounts.treasury_sol.to_account_info(),
                treasury_usdc: ctx.accounts.treasury_usdc.to_account_info(),
                destination_wallet: ctx.accounts.destination.to_account_info(),
                destination_token_account: ctx.accounts.destination_token_account.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            friend_groups::cpi::treasury_transfer(cpi_ctx, withdraw.sol_amount, withdraw.usdc_amount)?;
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


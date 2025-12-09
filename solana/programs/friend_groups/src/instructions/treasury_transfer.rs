use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;

pub fn handler(
    ctx: Context<crate::TreasuryTransfer>,
    sol_amount: u64,
    usdc_amount: u64,
) -> Result<()> {
    // This is a CPI-only instruction for authorized programs (like treasury)
    // The caller must be an authorized program
    
    // Validate amounts
    require!(
        sol_amount > 0 || usdc_amount > 0,
        FriendGroupError::InvalidAmount
    );
    
    let friend_group = &ctx.accounts.friend_group;
    
    // Transfer SOL if requested
    if sol_amount > 0 {
        // Validate treasury has sufficient balance
        let treasury_sol_balance = ctx.accounts.treasury_sol.lamports();
        require!(
            treasury_sol_balance >= sol_amount,
            FriendGroupError::InsufficientBalance
        );
        
        // Direct lamport manipulation - safe because treasury_sol is our PDA
        **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= sol_amount;
        **ctx.accounts.destination_wallet.to_account_info().try_borrow_mut_lamports()? += sol_amount;
    }
    
    // Transfer USDC if requested
    if usdc_amount > 0 {
        // Validate treasury has sufficient balance
        let treasury_usdc_balance = ctx.accounts.treasury_usdc.amount;
        require!(
            treasury_usdc_balance >= usdc_amount,
            FriendGroupError::InsufficientBalance
        );
        
        let friend_group_admin = friend_group.admin;
        let friend_group_bump = friend_group.friend_group_bump;
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_usdc.to_account_info(),
            to: ctx.accounts.destination_token_account.to_account_info(),
            authority: ctx.accounts.friend_group.to_account_info(),
        };
        
        let seeds = &[
            b"friend_group",
            friend_group_admin.as_ref(),
            &[friend_group_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        
        token::transfer(cpi_ctx, usdc_amount)?;
    }
    
    Ok(())
}

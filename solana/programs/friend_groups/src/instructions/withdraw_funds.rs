use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;

pub fn handler(ctx: Context<crate::friend_groups::WithdrawFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
    // Validate at least one amount > 0 (fail fast)
    require!(
        amount_sol > 0 || amount_usdc > 0,
        FriendGroupError::InvalidAmount
    );
    
    let member = &mut ctx.accounts.member;
    
    // Validate signer is the member
    require!(
        ctx.accounts.member_wallet.key() == member.user,
        FriendGroupError::Unauthorized
    );
    
    // Validate member belongs to this friend group
    require!(
        member.group == ctx.accounts.friend_group.key(),
        FriendGroupError::Unauthorized
    );
    
    // Can't withdraw if funds are locked (unless events resolved - handled separately)
    require!(
        !member.locked_funds,
        FriendGroupError::FundsLocked
    );
    
    // Withdraw SOL
    if amount_sol > 0 {
        require!(
            member.balance_sol >= amount_sol,
            FriendGroupError::InsufficientBalance
        );
        
        **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= amount_sol;
        **ctx.accounts.member_wallet.to_account_info().try_borrow_mut_lamports()? += amount_sol;
        
        member.balance_sol = member.balance_sol
            .checked_sub(amount_sol)
            .ok_or(FriendGroupError::InsufficientBalance)?;
    }
    
    // Withdraw USDC
    if amount_usdc > 0 {
        require!(
            member.balance_usdc >= amount_usdc,
            FriendGroupError::InsufficientBalance
        );
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_usdc.to_account_info(),
            to: ctx.accounts.member_usdc_account.to_account_info(),
            authority: ctx.accounts.friend_group.to_account_info(),
        };
        
        let seeds = &[
            b"friend_group",
            ctx.accounts.friend_group.admin.as_ref(),
            &[ctx.accounts.friend_group.friend_group_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        
        token::transfer(cpi_ctx, amount_usdc)?;
        
        member.balance_usdc = member.balance_usdc
            .checked_sub(amount_usdc)
            .ok_or(FriendGroupError::InsufficientBalance)?;
    }
    
    Ok(())
}


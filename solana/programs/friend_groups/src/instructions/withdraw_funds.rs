use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;

pub fn handler(ctx: Context<crate::friend_groups::WithdrawFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
    // Validate at least one amount > 0 (fail fast)
    require!(
        amount_sol > 0 || amount_usdc > 0,
        FriendGroupError::InvalidAmount
    );
    
    // Extract values before mutable borrow
    let friend_group_account_info = ctx.accounts.friend_group.to_account_info();
    let friend_group_key = ctx.accounts.friend_group.key();
    let treasury_bump = ctx.accounts.friend_group.treasury_bump;
    let friend_group_admin = ctx.accounts.friend_group.admin;
    let friend_group_bump = ctx.accounts.friend_group.friend_group_bump;
    
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
        
        // Direct lamport manipulation is safe here because:
        // 1. treasury_sol is validated by seeds in account constraints (seeds = [b"treasury_sol", friend_group.key().as_ref()])
        // 2. The seeds prove we control this PDA
        // 3. Even though it's System Program-owned, we can manipulate lamports of PDAs we control
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
            authority: friend_group_account_info,
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
        
        token::transfer(cpi_ctx, amount_usdc)?;
        
        member.balance_usdc = member.balance_usdc
            .checked_sub(amount_usdc)
            .ok_or(FriendGroupError::InsufficientBalance)?;
    }
    
    Ok(())
}


use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;

pub fn handler(ctx: Context<crate::friend_groups::DepositFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
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
    
    // Can't deposit if funds are locked
    require!(
        !member.locked_funds,
        FriendGroupError::FundsLocked
    );
    
    // Deposit SOL
    if amount_sol > 0 {
        // Use system program transfer (member is signer, so no PDA signing needed)
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.member_wallet.to_account_info(),
                    to: ctx.accounts.treasury_sol.to_account_info(),
                },
            ),
            amount_sol,
        )?;
        
        member.balance_sol = member.balance_sol
            .checked_add(amount_sol)
            .ok_or(FriendGroupError::InvalidAmount)?;
    }
    
    // Deposit USDC
    if amount_usdc > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.member_usdc_account.to_account_info(),
            to: ctx.accounts.treasury_usdc.to_account_info(),
            authority: ctx.accounts.member_wallet.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        
        token::transfer(cpi_ctx, amount_usdc)?;
        
        member.balance_usdc = member.balance_usdc
            .checked_add(amount_usdc)
            .ok_or(FriendGroupError::InvalidAmount)?;
    }
    
    Ok(())
}


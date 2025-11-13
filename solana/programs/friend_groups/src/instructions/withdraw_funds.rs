use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::*;

#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    #[account(mut)]
    pub friend_group: Account<'info, FriendGroup>,
    
    #[account(
        mut,
        seeds = [b"member", friend_group.key().as_ref(), member.user.key().as_ref()],
        bump
    )]
    pub member: Account<'info, GroupMember>,
    
    /// CHECK: SOL treasury PDA
    #[account(
        mut,
        seeds = [b"treasury_sol", friend_group.key().as_ref()],
        bump = friend_group.treasury_bump
    )]
    pub treasury_sol: SystemAccount<'info>,
    
    /// CHECK: USDC treasury token account
    #[account(mut)]
    pub treasury_usdc: Account<'info, TokenAccount>,
    
    /// CHECK: Member's USDC token account (destination)
    #[account(mut)]
    pub member_usdc_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub member_wallet: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<WithdrawFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
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
            friend_group.admin.as_ref(),
            &[friend_group.treasury_bump],
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


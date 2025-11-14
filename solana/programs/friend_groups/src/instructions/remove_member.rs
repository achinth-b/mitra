use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::*;

#[derive(Accounts)]
pub struct RemoveMember<'info> {
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
    
    /// CHECK: Member's wallet (for SOL refund)
    #[account(mut)]
    pub member_wallet: AccountInfo<'info>,
    
    /// CHECK: Member's USDC token account (for refund)
    #[account(mut)]
    pub member_usdc_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<RemoveMember>) -> Result<()> {
    // Get AccountInfo references before mutable borrow
    let friend_group_account_info = ctx.accounts.friend_group.to_account_info();
    let friend_group_admin = ctx.accounts.friend_group.admin;
    let friend_group_bump = ctx.accounts.friend_group.treasury_bump;
    
    let friend_group = &mut ctx.accounts.friend_group;
    let member = &ctx.accounts.member;
    
    // Only admin can remove members
    require!(
        friend_group.admin == ctx.accounts.admin.key(),
        FriendGroupError::Unauthorized
    );
    
    // Can't remove admin
    require!(
        member.role != MemberRole::Admin,
        FriendGroupError::Unauthorized
    );
    
    // Check minimum member requirement
    require!(
        friend_group.member_count > FriendGroup::MIN_MEMBERS,
        FriendGroupError::MinMembersRequired
    );
    
    // Validate member wallet matches the member being removed
    require!(
        ctx.accounts.member_wallet.key() == member.user,
        FriendGroupError::Unauthorized
    );
    
    // TODO: Check for active bets in events program
    let has_active_bets = false;
    
    if has_active_bets {
        let member_account = &mut ctx.accounts.member;
        member_account.locked_funds = true;
        
        friend_group.member_count = friend_group.member_count
            .checked_sub(1)
            .ok_or(FriendGroupError::MinMembersRequired)?;
        
        return Ok(());
    }
    
    // No active bets - proceed with full removal and refund
    let member = &ctx.accounts.member;
    
    // Refund SOL balance to member
    if member.balance_sol > 0 {
        **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= member.balance_sol;
        **ctx.accounts.member_wallet.to_account_info().try_borrow_mut_lamports()? += member.balance_sol;
    }
    
    // Refund USDC balance
    if member.balance_usdc > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_usdc.to_account_info(),
            to: ctx.accounts.member_usdc_account.to_account_info(),
            authority: friend_group_account_info, // Use the AccountInfo we got earlier
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
        
        token::transfer(cpi_ctx, member.balance_usdc)?;
    }
    
    // Decrement member count
    friend_group.member_count = friend_group.member_count
        .checked_sub(1)
        .ok_or(FriendGroupError::MinMembersRequired)?;
    
    // Close member account and refund rent
    let member_account_info = ctx.accounts.member.to_account_info();
    let member_wallet_info = ctx.accounts.member_wallet.to_account_info();
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(member_account_info.data_len());
    
    **member_account_info.try_borrow_mut_lamports()? -= rent_lamports;
    **member_wallet_info.try_borrow_mut_lamports()? += rent_lamports;
    member_account_info.assign(&system_program::ID);
    member_account_info.resize(0)?;
    
    Ok(())
}
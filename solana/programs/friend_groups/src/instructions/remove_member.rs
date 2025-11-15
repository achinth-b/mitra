use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::errors::*;
use crate::state::{FriendGroup, MemberRole};

pub fn handler(ctx: Context<crate::friend_groups::RemoveMember>) -> Result<()> {
    // Get AccountInfo references and values before mutable borrow
    let friend_group_account_info = ctx.accounts.friend_group.to_account_info();
    let friend_group_admin = ctx.accounts.friend_group.admin;
    let friend_group_bump = ctx.accounts.friend_group.friend_group_bump;
    let friend_group_key = ctx.accounts.friend_group.key();
    let treasury_bump = ctx.accounts.friend_group.treasury_bump;
    
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
        // Direct lamport manipulation is safe here because:
        // 1. treasury_sol is validated by seeds in account constraints (seeds = [b"treasury_sol", friend_group.key().as_ref()])
        // 2. The seeds prove we control this PDA
        // 3. Even though it's System Program-owned, we can manipulate lamports of PDAs we control
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
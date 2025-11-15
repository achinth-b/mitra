use anchor_lang::prelude::*;
use crate::errors::*;

pub fn handler(ctx: Context<crate::friend_groups::CreateGroup>, name: String) -> Result<()> {
    require!(name.len() <= 50, FriendGroupError::NameTooLong);
    
    // Validate treasury_usdc is owned by the friend_group PDA
    require!(
        ctx.accounts.treasury_usdc.owner == ctx.accounts.friend_group.key(),
        FriendGroupError::Unauthorized
    );
    
    let friend_group = &mut ctx.accounts.friend_group;
    let clock = Clock::get()?;
    
    friend_group.admin = ctx.accounts.admin.key();
    friend_group.name = name;
    friend_group.member_count = 1; // Admin is first member
    friend_group.treasury_sol = ctx.accounts.treasury_sol.key();
    friend_group.treasury_usdc = ctx.accounts.treasury_usdc.key();
    friend_group.treasury_bump = ctx.bumps.treasury_sol;
    friend_group.friend_group_bump = ctx.bumps.friend_group;
    friend_group.created_at = clock.unix_timestamp;
    
    Ok(())
}
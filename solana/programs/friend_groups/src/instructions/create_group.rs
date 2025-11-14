use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::*;
use crate::errors::*;

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateGroup<'info> {
    #[account(
        init,
        payer = admin,
        space = FriendGroup::MAX_SIZE,
        seeds = [b"friend_group", admin.key().as_ref()],
        bump
    )]
    pub friend_group: Account<'info, FriendGroup>,
    
    /// CHECK: PDA for SOL treasury, validated by seeds
    /// Using UncheckedAccount since we're creating a new system account
    #[account(
        init,
        payer = admin,
        space = 8, // Minimum space for system account
        seeds = [b"treasury_sol", friend_group.key().as_ref()],
        bump
    )]
    pub treasury_sol: UncheckedAccount<'info>,
    
    /// USDC treasury token account - must be created as ATA for friend_group PDA before this instruction
    /// The owner will be validated in the handler after friend_group PDA is derived
    #[account(
        constraint = treasury_usdc.mint == usdc_mint.key(),
    )]
    pub treasury_usdc: Account<'info, TokenAccount>,
    
    /// CHECK: USDC mint address
    pub usdc_mint: AccountInfo<'info>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateGroup>, name: String) -> Result<()> {
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
    friend_group.treasury_bump = ctx.bumps.friend_group;
    friend_group.created_at = clock.unix_timestamp;
    
    Ok(())
}
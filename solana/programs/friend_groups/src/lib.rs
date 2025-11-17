use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

pub mod state;
pub mod errors;
pub mod instructions;

use state::*;

declare_id!("A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ");

#[program]
pub mod friend_groups {
    use super::*;

    // ============================================================================
    // CREATE GROUP
    // ============================================================================
    
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

    pub fn create_group(ctx: Context<CreateGroup>, name: String) -> Result<()> {
        instructions::create_group::handler(ctx, name)
    }

    // ============================================================================
    // INVITE MEMBER
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct InviteMember<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        #[account(
            init,
            payer = inviter,
            space = Invite::MAX_SIZE,
            seeds = [b"invite", friend_group.key().as_ref(), invited_user.key().as_ref()],
            bump
        )]
        pub invite: Account<'info, Invite>,
        
        /// CHECK: User being invited
        pub invited_user: AccountInfo<'info>,
        
        #[account(mut)]
        pub inviter: Signer<'info>,
        
        pub system_program: Program<'info, System>,
    }

    pub fn invite_member(ctx: Context<InviteMember>) -> Result<()> {
        instructions::invite_member::handler(ctx)
    }

    // ============================================================================
    // ACCEPT INVITE
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct AcceptInvite<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        #[account(
            mut,
            close = invited_user, // Close invite account and refund rent to user
            seeds = [b"invite", friend_group.key().as_ref(), invited_user.key().as_ref()],
            bump
        )]
        pub invite: Account<'info, Invite>,
        
        #[account(
            init,
            payer = invited_user,
            space = GroupMember::MAX_SIZE,
            seeds = [b"member", friend_group.key().as_ref(), invited_user.key().as_ref()],
            bump
        )]
        pub group_member: Account<'info, GroupMember>,
        
        #[account(mut)]
        pub invited_user: Signer<'info>,
        
        pub system_program: Program<'info, System>,
    }

    pub fn accept_invite(ctx: Context<AcceptInvite>) -> Result<()> {
        instructions::accept_invite::handler(ctx)
    }

    // ============================================================================
    // REMOVE MEMBER
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct RemoveMember<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        /// CHECK: Member's wallet (for SOL refund)
        #[account(mut)]
        pub member_wallet: AccountInfo<'info>,
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        /// CHECK: SOL treasury PDA (validated by seeds, owned by System Program)
        #[account(
            mut,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump = friend_group.treasury_bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: Member's USDC token account (for refund)
        #[account(mut)]
        pub member_usdc_account: Account<'info, TokenAccount>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn remove_member(ctx: Context<RemoveMember>) -> Result<()> {
        instructions::remove_member::handler(ctx)
    }

    // ============================================================================
    // DEPOSIT FUNDS
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct DepositFunds<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        /// CHECK: SOL treasury PDA (validated by seeds, owned by System Program)
        #[account(
            mut,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump = friend_group.treasury_bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: Member's USDC token account (source)
        #[account(mut)]
        pub member_usdc_account: Account<'info, TokenAccount>,
        
        #[account(mut)]
        pub member_wallet: Signer<'info>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn deposit_funds(
        ctx: Context<DepositFunds>,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> Result<()> {
        instructions::deposit_funds::handler(ctx, amount_sol, amount_usdc)
    }

    // ============================================================================
    // WITHDRAW FUNDS
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct WithdrawFunds<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        /// CHECK: SOL treasury PDA (validated by seeds, owned by System Program)
        #[account(
            mut,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump = friend_group.treasury_bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
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

    pub fn withdraw_funds(
        ctx: Context<WithdrawFunds>,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> Result<()> {
        instructions::withdraw_funds::handler(ctx, amount_sol, amount_usdc)
    }

    // ============================================================================
    // TREASURY TRANSFER (CPI only)
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct TreasuryTransfer<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        /// CHECK: SOL treasury PDA (validated by seeds)
        #[account(
            mut,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump = friend_group.treasury_bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: Destination wallet for SOL
        #[account(mut)]
        pub destination_wallet: UncheckedAccount<'info>,
        
        /// CHECK: Destination token account for USDC
        #[account(mut)]
        pub destination_token_account: Account<'info, TokenAccount>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }
    
    pub fn treasury_transfer(
        ctx: Context<TreasuryTransfer>,
        sol_amount: u64,
        usdc_amount: u64,
    ) -> Result<()> {
        instructions::treasury_transfer::handler(ctx, sol_amount, usdc_amount)
    }
}

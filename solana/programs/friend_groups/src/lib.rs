use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

pub mod state;
pub mod errors;

use state::*;
use errors::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

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
            space = 8,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// USDC treasury token account - must be created as ATA for friend_group PDA before this instruction
        #[account(constraint = treasury_usdc.mint == usdc_mint.key())]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: USDC mint address
        pub usdc_mint: AccountInfo<'info>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_group(ctx: Context<CreateGroup>, name: String) -> Result<()> {
        require!(name.len() <= 50, FriendGroupError::NameTooLong);
        
        require!(
            ctx.accounts.treasury_usdc.owner == ctx.accounts.friend_group.key(),
            FriendGroupError::Unauthorized
        );
        
        let friend_group = &mut ctx.accounts.friend_group;
        let clock = Clock::get()?;
        
        friend_group.admin = ctx.accounts.admin.key();
        friend_group.name = name;
        friend_group.member_count = 1;
        friend_group.treasury_sol = ctx.accounts.treasury_sol.key();
        friend_group.treasury_usdc = ctx.accounts.treasury_usdc.key();
        friend_group.treasury_bump = ctx.bumps.treasury_sol;
        friend_group.friend_group_bump = ctx.bumps.friend_group;
        friend_group.created_at = clock.unix_timestamp;
        
        Ok(())
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
        let friend_group = &ctx.accounts.friend_group;
        let clock = Clock::get()?;
        
        require!(
            friend_group.admin == ctx.accounts.inviter.key(),
            FriendGroupError::Unauthorized
        );
        
        require!(
            friend_group.member_count < FriendGroup::MAX_MEMBERS,
            FriendGroupError::MaxMembersReached
        );
        
        require!(
            ctx.accounts.invited_user.key() != ctx.accounts.inviter.key(),
            FriendGroupError::InvalidAmount
        );
        
        let invite = &mut ctx.accounts.invite;
        invite.group = friend_group.key();
        invite.invited_user = ctx.accounts.invited_user.key();
        invite.inviter = ctx.accounts.inviter.key();
        invite.created_at = clock.unix_timestamp;
        invite.expires_at = clock.unix_timestamp + Invite::EXPIRY_SECONDS;
        
        Ok(())
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
            close = invited_user,
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
        let invite = &ctx.accounts.invite;
        let clock = Clock::get()?;
        
        require!(
            clock.unix_timestamp < invite.expires_at,
            FriendGroupError::InviteExpired
        );
        
        require!(
            ctx.accounts.invited_user.key() == invite.invited_user,
            FriendGroupError::Unauthorized
        );
        
        let friend_group = &mut ctx.accounts.friend_group;
        
        require!(
            friend_group.member_count < FriendGroup::MAX_MEMBERS,
            FriendGroupError::MaxMembersReached
        );
        
        let group_member = &mut ctx.accounts.group_member;
        group_member.user = ctx.accounts.invited_user.key();
        group_member.group = friend_group.key();
        group_member.role = MemberRole::Member;
        group_member.balance_sol = 0;
        group_member.balance_usdc = 0;
        group_member.locked_funds = false;
        group_member.joined_at = clock.unix_timestamp;
        
        friend_group.member_count = friend_group.member_count
            .checked_add(1)
            .ok_or(FriendGroupError::MaxMembersReached)?;
        
        Ok(())
    }

    // ============================================================================
    // REMOVE MEMBER
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct RemoveMember<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        /// CHECK: SOL treasury PDA
        #[account(
            mut,
            seeds = [b"treasury_sol", friend_group.key().as_ref()],
            bump = friend_group.treasury_bump
        )]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: Member's wallet (for SOL refund)
        #[account(mut)]
        pub member_wallet: AccountInfo<'info>,
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        /// CHECK: Member's USDC token account (for refund)
        #[account(mut)]
        pub member_usdc_account: Account<'info, TokenAccount>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn remove_member(ctx: Context<RemoveMember>) -> Result<()> {
        let friend_group_account_info = ctx.accounts.friend_group.to_account_info();
        let friend_group_admin = ctx.accounts.friend_group.admin;
        let friend_group_bump = ctx.accounts.friend_group.friend_group_bump;
        
        let friend_group = &mut ctx.accounts.friend_group;
        let member = &ctx.accounts.member;
        
        require!(
            friend_group.admin == ctx.accounts.admin.key(),
            FriendGroupError::Unauthorized
        );
        
        require!(
            member.role != MemberRole::Admin,
            FriendGroupError::Unauthorized
        );
        
        require!(
            friend_group.member_count > FriendGroup::MIN_MEMBERS,
            FriendGroupError::MinMembersRequired
        );
        
        require!(
            ctx.accounts.member_wallet.key() == member.user,
            FriendGroupError::Unauthorized
        );
        
        let has_active_bets = false;
        
        if has_active_bets {
            let member_account = &mut ctx.accounts.member;
            member_account.locked_funds = true;
            
            friend_group.member_count = friend_group.member_count
                .checked_sub(1)
                .ok_or(FriendGroupError::MinMembersRequired)?;
            
            return Ok(());
        }
        
        let member = &ctx.accounts.member;
        
        if member.balance_sol > 0 {
            // Transfer SOL from treasury PDA to member wallet
            // Use direct lamport manipulation since treasury_sol is a PDA-owned account
            **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= member.balance_sol;
            **ctx.accounts.member_wallet.to_account_info().try_borrow_mut_lamports()? += member.balance_sol;
        }
        
        if member.balance_usdc > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.treasury_usdc.to_account_info(),
                to: ctx.accounts.member_usdc_account.to_account_info(),
                authority: friend_group_account_info,
            };
            
            // Use friend_group PDA seeds for signing
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
        
        friend_group.member_count = friend_group.member_count
            .checked_sub(1)
            .ok_or(FriendGroupError::MinMembersRequired)?;
        
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

    // ============================================================================
    // DEPOSIT FUNDS
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct DepositFunds<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        /// CHECK: SOL treasury PDA
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
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn deposit_funds(ctx: Context<DepositFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
        require!(
            amount_sol > 0 || amount_usdc > 0,
            FriendGroupError::InvalidAmount
        );
        
        let member = &mut ctx.accounts.member;
        
        require!(
            ctx.accounts.member_wallet.key() == member.user,
            FriendGroupError::Unauthorized
        );
        
        require!(
            member.group == ctx.accounts.friend_group.key(),
            FriendGroupError::Unauthorized
        );
        
        require!(
            !member.locked_funds,
            FriendGroupError::FundsLocked
        );
        
        if amount_sol > 0 {
            // Transfer SOL from member wallet to treasury PDA using system program
            // No signing needed - member_wallet is the signer
            anchor_lang::solana_program::program::invoke(
                &anchor_lang::solana_program::system_instruction::transfer(
                    ctx.accounts.member_wallet.key,
                    ctx.accounts.treasury_sol.key,
                    amount_sol,
                ),
                &[
                    ctx.accounts.member_wallet.to_account_info(),
                    ctx.accounts.treasury_sol.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
            
            member.balance_sol = member.balance_sol
                .checked_add(amount_sol)
                .ok_or(FriendGroupError::InvalidAmount)?;
        }
        
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

    // ============================================================================
    // WITHDRAW FUNDS
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct WithdrawFunds<'info> {
        #[account(mut)]
        pub friend_group: Account<'info, FriendGroup>,
        
        /// CHECK: SOL treasury PDA
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
        
        #[account(
            mut,
            seeds = [b"member", friend_group.key().as_ref(), member_wallet.key().as_ref()],
            bump
        )]
        pub member: Account<'info, GroupMember>,
        
        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    pub fn withdraw_funds(ctx: Context<WithdrawFunds>, amount_sol: u64, amount_usdc: u64) -> Result<()> {
        require!(
            amount_sol > 0 || amount_usdc > 0,
            FriendGroupError::InvalidAmount
        );
        
        let member = &mut ctx.accounts.member;
        
        require!(
            ctx.accounts.member_wallet.key() == member.user,
            FriendGroupError::Unauthorized
        );
        
        require!(
            member.group == ctx.accounts.friend_group.key(),
            FriendGroupError::Unauthorized
        );
        
        require!(
            !member.locked_funds,
            FriendGroupError::FundsLocked
        );
        
        if amount_sol > 0 {
            require!(
                member.balance_sol >= amount_sol,
                FriendGroupError::InsufficientBalance
            );
            
            // Transfer SOL from treasury PDA to member wallet
            // Use direct lamport manipulation since treasury_sol is a PDA-owned account
            **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= amount_sol;
            **ctx.accounts.member_wallet.to_account_info().try_borrow_mut_lamports()? += amount_sol;
            
            member.balance_sol = member.balance_sol
                .checked_sub(amount_sol)
                .ok_or(FriendGroupError::InsufficientBalance)?;
        }
        
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
            
            // Extract bump before mutable borrow
            let friend_group_admin = ctx.accounts.friend_group.admin;
            let friend_group_bump = ctx.accounts.friend_group.friend_group_bump;
            
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
}

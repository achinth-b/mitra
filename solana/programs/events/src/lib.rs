use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use friend_groups::state::FriendGroup;
use sha3::{Keccak256, Digest};

pub mod state;
pub mod errors;

use state::*;
use errors::*;

declare_id!("GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F"); // Generate with: anchor keys list

// Backend authority PDA seeds - backend can commit state updates
pub const BACKEND_AUTHORITY_SEED: &[u8] = b"backend_authority";

#[program]
pub mod events {
    use super::*;

    // ============================================================================
    // CREATE EVENT
    // ============================================================================
    
    #[derive(Accounts)]
    #[instruction(title: String)]
    pub struct CreateEvent<'info> {
        #[account(
            init,
            payer = admin,
            space = EventContract::MAX_SIZE,
            seeds = [
                b"event",
                group.key().as_ref(),
                &Keccak256::digest(title.as_bytes())[..]
            ],
            bump
        )]
        pub event_contract: Account<'info, EventContract>,
        
        #[account(
            init,
            payer = admin,
            space = EventState::MAX_SIZE,
            seeds = [b"event_state", event_contract.key().as_ref()],
            bump
        )]
        pub event_state: Account<'info, EventState>,
        
        /// CHECK: Friend group account
        #[account(mut)]
        pub group: Account<'info, FriendGroup>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        pub system_program: Program<'info, System>,
    }

    pub fn create_event(
        ctx: Context<CreateEvent>,
        title: String,
        description: String,
        outcomes: Vec<String>,
        settlement_type: SettlementType,
        resolve_by: i64,
    ) -> Result<()> {
        require!(title.len() <= 100, EventError::TitleTooLong);
        require!(description.len() <= 500, EventError::DescriptionTooLong);
        require!(outcomes.len() >= 2 && outcomes.len() <= 10, EventError::TooManyOutcomes);
        
        for outcome in &outcomes {
            require!(outcome.len() <= 50, EventError::InvalidOutcome);
        }
        
        let clock = Clock::get()?;
        require!(resolve_by > clock.unix_timestamp, EventError::InvalidResolveBy);
        
        let group = &ctx.accounts.group;
        require!(group.admin == ctx.accounts.admin.key(), EventError::Unauthorized);
        
        let event_contract = &mut ctx.accounts.event_contract;
        event_contract.event_id = event_contract.key();
        event_contract.group = ctx.accounts.group.key();
        event_contract.title = title;
        event_contract.description = description;
        event_contract.outcomes = outcomes;
        event_contract.settlement_type = settlement_type;
        event_contract.status = EventStatus::Active;
        event_contract.resolve_by = resolve_by;
        event_contract.total_volume = 0;
        event_contract.created_at = clock.unix_timestamp;
        event_contract.settled_at = None;
        event_contract.winning_outcome = None;
        
        let event_state = &mut ctx.accounts.event_state;
        event_state.event = ctx.accounts.event_contract.key();
        event_state.last_merkle_root = [0u8; 32];
        event_state.last_commit_slot = 0;
        event_state.total_liquidity = 0;
        
        Ok(())
    }

    // ============================================================================
    // COMMIT STATE
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct CommitState<'info> {
        #[account(mut)]
        pub event_contract: Account<'info, EventContract>,
        
        #[account(
            mut,
            seeds = [b"event_state", event_contract.key().as_ref()],
            bump
        )]
        pub event_state: Account<'info, EventState>,
        
        /// CHECK: Backend authority PDA (validated by seeds)
        #[account(
            seeds = [BACKEND_AUTHORITY_SEED],
            bump
        )]
        pub backend_authority: UncheckedAccount<'info>,
    }

    pub fn commit_state(
        ctx: Context<CommitState>,
        merkle_root: [u8; 32],
    ) -> Result<()> {
        // Backend authority is validated by PDA seeds constraint above
        require!(
            ctx.accounts.event_contract.status == EventStatus::Active,
            EventError::EventAlreadySettled
        );
        
        let clock = Clock::get()?;
        let slot = clock.slot;
        
        let event_state = &mut ctx.accounts.event_state;
        event_state.last_merkle_root = merkle_root;
        event_state.last_commit_slot = slot;
        
        Ok(())
    }

    // ============================================================================
    // SETTLE EVENT
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct SettleEvent<'info> {
        #[account(mut)]
        pub event_contract: Account<'info, EventContract>,
        
        /// CHECK: Friend group account
        pub group: Account<'info, FriendGroup>,
        
        pub admin: Signer<'info>,
    }

    pub fn settle_event(
        ctx: Context<SettleEvent>,
        winning_outcome: String,
    ) -> Result<()> {
        let event = &mut ctx.accounts.event_contract;
        
        require!(
            event.status == EventStatus::Active,
            EventError::EventAlreadySettled
        );
        
        require!(
            ctx.accounts.group.admin == ctx.accounts.admin.key(),
            EventError::Unauthorized
        );
        
        require!(
            event.group == ctx.accounts.group.key(),
            EventError::Unauthorized
        );
        
        // Verify winning_outcome is in the outcomes list
        require!(
            event.outcomes.contains(&winning_outcome),
            EventError::InvalidOutcome
        );
        
        let clock = Clock::get()?;
        
        event.status = EventStatus::Resolved;
        event.settled_at = Some(clock.unix_timestamp);
        event.winning_outcome = Some(winning_outcome);
        
        Ok(())
    }

    // ============================================================================
    // CLAIM WINNINGS
    // ============================================================================
    
    #[derive(Accounts)]
    pub struct ClaimWinnings<'info> {
        #[account(
            mut,
            constraint = event_contract.status == EventStatus::Resolved @ EventError::EventNotSettled,
            constraint = event_contract.group == group.key() @ EventError::Unauthorized
        )]
        pub event_contract: Account<'info, EventContract>,
        
        /// Friend group account - validates event belongs to this group
        pub group: Account<'info, FriendGroup>,
        
        /// USDC treasury token account owned by the friend group PDA
        #[account(
            mut,
            constraint = treasury_usdc.owner == group.key() @ EventError::InvalidTreasury
        )]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// User's USDC token account (destination for winnings)
        #[account(
            mut,
            constraint = user_usdc_account.mint == treasury_usdc.mint @ EventError::InvalidMint
        )]
        pub user_usdc_account: Account<'info, TokenAccount>,
        
        /// Member account verifying user is a group member
        #[account(
            seeds = [b"member", group.key().as_ref(), user.key().as_ref()],
            bump,
            constraint = member.data_len() > 0 @ EventError::NotGroupMember
        )]
        pub member: AccountInfo<'info>,
        
        #[account(mut)]
        pub user: Signer<'info>,
        
        pub token_program: Program<'info, Token>,
    }

    pub fn claim_winnings(
        ctx: Context<ClaimWinnings>,
        amount: u64,
    ) -> Result<()> {
        // Input validation (constraints handle account validation)
        require!(amount > 0, EventError::ZeroAmount);
        
        // Validate treasury has sufficient balance
        require!(
            ctx.accounts.treasury_usdc.amount >= amount,
            EventError::InsufficientWinnings
        );
        
        // Transfer USDC from treasury to user
        // Note: The friend_group PDA is the authority for the treasury
        let seeds = &[
            b"friend_group",
            ctx.accounts.group.admin.as_ref(),
            &[ctx.accounts.group.friend_group_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_usdc.to_account_info(),
            to: ctx.accounts.user_usdc_account.to_account_info(),
            authority: ctx.accounts.group.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        
        token::transfer(cpi_ctx, amount)?;
        
        Ok(())
    }
}

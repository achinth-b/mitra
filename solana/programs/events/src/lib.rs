use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use friend_groups::state::FriendGroup;
use sha3::{Keccak256, Digest};

pub mod state;
pub mod errors;

use state::*;
use errors::*;

declare_id!("GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F"); // Generate with: anchor keys list

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
        
        /// CHECK: Backend authority (should be a PDA or specific pubkey)
        pub backend_authority: Signer<'info>,
    }

    pub fn commit_state(
        ctx: Context<CommitState>,
        merkle_root: [u8; 32],
    ) -> Result<()> {
        // TODO: Add backend authority check
        // For now, we'll add a constant backend pubkey or PDA
        // require!(
        //     ctx.accounts.backend_authority.key() == BACKEND_AUTHORITY,
        //     EventError::NotBackendAuthority
        // );
        
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
        #[account(mut)]
        pub event_contract: Account<'info, EventContract>,
        
        /// CHECK: Friend group account
        pub group: Account<'info, FriendGroup>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, TokenAccount>,
        
        /// CHECK: User's USDC token account (destination)
        #[account(mut)]
        pub user_usdc_account: Account<'info, TokenAccount>,
        
        /// CHECK: Member account (we'll verify it exists)
        #[account(
            seeds = [b"member", group.key().as_ref(), user.key().as_ref()],
            bump
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
        let event = &ctx.accounts.event_contract;
        
        require!(
            event.status == EventStatus::Resolved,
            EventError::EventNotSettled
        );
        
        require!(
            event.group == ctx.accounts.group.key(),
            EventError::Unauthorized
        );
        
        require!(amount > 0, EventError::InsufficientWinnings);
        
        // Transfer USDC from treasury to user
        let group_account_info = ctx.accounts.group.to_account_info();
        let seeds = &[
            b"friend_group",
            ctx.accounts.group.admin.as_ref(),
            &[ctx.accounts.group.treasury_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_usdc.to_account_info(),
            to: ctx.accounts.user_usdc_account.to_account_info(),
            authority: group_account_info,
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
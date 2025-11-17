use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod instructions;

use state::*;

declare_id!("38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB");

#[program]
pub mod treasury {
    use super::*;

    // ============================================================================
    // BATCH SETTLE
    // ============================================================================
    
    #[derive(Accounts)]
    #[instruction(batch_id: u64, settlements: Vec<SettlementEntry>)]
    pub struct BatchSettle<'info> {
        #[account(
            init_if_needed,
            payer = admin,
            space = BatchSettlement::MAX_SIZE,
            seeds = [b"batch_settlement", friend_group.key().as_ref(), batch_id.to_le_bytes().as_ref()],
            bump
        )]
        pub batch_settlement: Account<'info, BatchSettlement>,
        
        #[account(mut)]
        pub friend_group: Account<'info, friend_groups::state::FriendGroup>,
        
        /// CHECK: SOL treasury PDA (validated by seeds from friend_groups program)
        /// We validate it matches friend_group.treasury_sol
        #[account(mut)]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, anchor_spl::token::TokenAccount>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        /// CHECK: friend_groups program for CPI
        pub friend_groups_program: AccountInfo<'info>,
        pub token_program: Program<'info, anchor_spl::token::Token>,
        pub system_program: Program<'info, System>,
    }
    
    pub fn batch_settle(
        ctx: Context<BatchSettle>,
        batch_id: u64,
        settlements: Vec<SettlementEntry>,
    ) -> Result<()> {
        instructions::batch_settle_handler(ctx, batch_id, settlements)
    }

    // ============================================================================
    // EMERGENCY WITHDRAW
    // ============================================================================
    
    #[derive(Accounts)]
    #[instruction(request_id: u64, sol_amount: u64, usdc_amount: u64)]
    pub struct EmergencyWithdrawAccounts<'info> {
        #[account(
            init_if_needed,
            payer = admin,
            space = EmergencyWithdraw::MAX_SIZE,
            seeds = [b"emergency_withdraw", friend_group.key().as_ref(), request_id.to_le_bytes().as_ref()],
            bump
        )]
        pub emergency_withdraw: Account<'info, EmergencyWithdraw>,
        
        #[account(
            mut,
            constraint = friend_group.admin == admin.key() @ errors::TreasuryError::Unauthorized
        )]
        pub friend_group: Account<'info, friend_groups::state::FriendGroup>,
        
        /// CHECK: SOL treasury PDA (validated by seeds from friend_groups program)
        /// We validate it matches friend_group.treasury_sol
        #[account(mut)]
        pub treasury_sol: UncheckedAccount<'info>,
        
        /// CHECK: USDC treasury token account
        #[account(mut)]
        pub treasury_usdc: Account<'info, anchor_spl::token::TokenAccount>,
        
        /// CHECK: Destination wallet for SOL
        #[account(mut)]
        pub destination: UncheckedAccount<'info>,
        
        /// CHECK: Destination token account for USDC
        #[account(mut)]
        pub destination_token_account: Account<'info, anchor_spl::token::TokenAccount>,
        
        #[account(mut)]
        pub admin: Signer<'info>,
        
        /// CHECK: friend_groups program for CPI
        pub friend_groups_program: AccountInfo<'info>,
        pub token_program: Program<'info, anchor_spl::token::Token>,
        pub system_program: Program<'info, System>,
    }
    
    pub fn emergency_withdraw(
        ctx: Context<EmergencyWithdrawAccounts>,
        request_id: u64,
        sol_amount: u64,
        usdc_amount: u64,
    ) -> Result<()> {
        instructions::emergency_withdraw_handler(ctx, request_id, sol_amount, usdc_amount)
    }
}


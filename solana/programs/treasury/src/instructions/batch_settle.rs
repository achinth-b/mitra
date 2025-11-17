use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::spl_token::instruction as token_instruction;
use crate::errors::*;
use crate::state::{BatchSettlement, SettlementEntry, BatchStatus, TokenType};
use friend_groups::state::FriendGroup;


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
    
    #[account(
        mut,
        constraint = friend_group.treasury_sol == treasury_sol.key() @ TreasuryError::InvalidFriendGroup,
        constraint = friend_group.treasury_usdc == treasury_usdc.key() @ TreasuryError::InvalidFriendGroup
    )]
    pub friend_group: Account<'info, FriendGroup>,
    
    /// CHECK: SOL treasury PDA (validated by seeds)
    #[account(
        mut,
        seeds = [b"treasury_sol", friend_group.key().as_ref()],
        bump = friend_group.treasury_bump
    )]
    pub treasury_sol: UncheckedAccount<'info>,
    
    /// CHECK: USDC treasury token account
    #[account(
        mut,
        constraint = treasury_usdc.owner == friend_group.key() @ TreasuryError::InvalidTreasury
    )]
    pub treasury_usdc: Account<'info, anchor_spl::token::TokenAccount>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<crate::treasury::BatchSettle>,
    batch_id: u64,
    settlements: Vec<SettlementEntry>,
) -> Result<()> {
    require!(
        settlements.len() > 0 && settlements.len() <= BatchSettlement::MAX_SETTLEMENTS_PER_BATCH,
        TreasuryError::InvalidSettlement
    );
    
    let batch = &mut ctx.accounts.batch_settlement;
    let friend_group = &ctx.accounts.friend_group;
    let clock = Clock::get()?;
    
    // Validate admin
    require!(
        friend_group.admin == ctx.accounts.admin.key(),
        TreasuryError::Unauthorized
    );
    
    // Initialize batch if it's new
    if batch.batch_id == 0 {
        batch.batch_id = batch_id;
        batch.friend_group = ctx.accounts.friend_group.key();
        batch.status = BatchStatus::Pending;
        batch.created_at = clock.unix_timestamp;
        batch.executed_at = None;
        batch.total_sol_amount = 0;
        batch.total_usdc_amount = 0;
    }
    
    // Validate friend group matches
    require!(
        batch.friend_group == ctx.accounts.friend_group.key(),
        TreasuryError::InvalidFriendGroup
    );
    
    // Validate batch is pending
    require!(
        batch.status == BatchStatus::Pending,
        TreasuryError::BatchAlreadyExecuted
    );
    
    // Calculate total amounts
    let mut total_sol = 0u64;
    let mut total_usdc = 0u64;
    
    for entry in &settlements {
        require!(entry.amount > 0, TreasuryError::InvalidAmount);
        
        match entry.token_type {
            TokenType::Sol => {
                total_sol = total_sol
                    .checked_add(entry.amount)
                    .ok_or(TreasuryError::InvalidAmount)?;
            }
            TokenType::Usdc => {
                total_usdc = total_usdc
                    .checked_add(entry.amount)
                    .ok_or(TreasuryError::InvalidAmount)?;
            }
        }
    }
    
    // Validate treasury has sufficient balance
    let treasury_sol_balance = ctx.accounts.treasury_sol.lamports();
    let treasury_usdc_balance = ctx.accounts.treasury_usdc.amount;
    
    require!(
        treasury_sol_balance >= total_sol,
        TreasuryError::InsufficientBalance
    );
    
    require!(
        treasury_usdc_balance >= total_usdc,
        TreasuryError::InsufficientBalance
    );
    
    // Process settlements using remaining_accounts
    // Remaining accounts should be: [user_wallet_1, user_token_account_1, user_wallet_2, user_token_account_2, ...]
    // For each settlement entry, we need 2 accounts: wallet (for SOL) and token account (for USDC)
    let remaining_accounts = ctx.remaining_accounts;
    let expected_accounts = settlements.len() * 2; // Each settlement needs wallet + token account
    
    require!(
        remaining_accounts.len() == expected_accounts,
        TreasuryError::InvalidSettlement
    );
    
    let friend_group_admin = friend_group.admin;
    let friend_group_bump = ctx.accounts.friend_group.friend_group_bump;
    let seeds = &[
        b"friend_group",
        friend_group_admin.as_ref(),
        &[friend_group_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    // Process each settlement
    for (idx, entry) in settlements.iter().enumerate() {
        let wallet_idx = idx * 2;
        let token_idx = idx * 2 + 1;
        
        let user_wallet = &remaining_accounts[wallet_idx];
        let user_token_account = &remaining_accounts[token_idx];
        
        // Validate user matches entry
        require!(
            user_wallet.key() == entry.user,
            TreasuryError::InvalidSettlement
        );
        
        // Validate token account for USDC transfers
        if entry.token_type == TokenType::Usdc && entry.amount > 0 {
            // Verify token account is owned by token program
            require!(
                *user_token_account.owner == ctx.accounts.token_program.key(),
                TreasuryError::InvalidSettlement
            );
            
            // Verify token account mint matches treasury mint
            // We need to deserialize the token account to check mint
            let token_account_data = user_token_account.try_borrow_data()?;
            let token_account = anchor_spl::token::TokenAccount::try_deserialize(&mut &token_account_data[..])?;
            require!(
                token_account.mint == ctx.accounts.treasury_usdc.mint,
                TreasuryError::InvalidTokenAccount
            );
        }
        
        // Process SOL transfer
        if entry.token_type == TokenType::Sol && entry.amount > 0 {
            **ctx.accounts.treasury_sol.to_account_info().try_borrow_mut_lamports()? -= entry.amount;
            **user_wallet.try_borrow_mut_lamports()? += entry.amount;
        }
        
        // Process USDC transfer  
        if entry.token_type == TokenType::Usdc && entry.amount > 0 {
            // Use invoke_signed with manually constructed instruction
            let transfer_ix = token_instruction::transfer(
                &ctx.accounts.token_program.key(),
                &ctx.accounts.treasury_usdc.key(),
                user_token_account.key,
                &ctx.accounts.friend_group.key(),
                &[],
                entry.amount,
            )?;
            
            // Use invoke_signed - AccountInfo is invariant over lifetime, so we need unsafe to unify
            // SAFETY: All AccountInfos come from the same Context<'info>, so lifetimes are actually the same
            // Rust's type system just can't prove it due to variance rules
            let treasury_ai = ctx.accounts.treasury_usdc.to_account_info();
            let user_token_cloned = user_token_account.clone();
            let friend_group_ai = ctx.accounts.friend_group.to_account_info();
            let token_program_ai = ctx.accounts.token_program.to_account_info();
            
            // Transmute the cloned AccountInfo to match the lifetime of ctx.accounts AccountInfos
            // This is safe because all AccountInfos originate from the same Context<'info>
            // We need to transmute through a raw pointer to change the lifetime parameter
            let user_token_ai = unsafe {
                // Get a raw pointer to the cloned AccountInfo
                let ptr = &user_token_cloned as *const AccountInfo;
                // Transmute the pointer to change the lifetime (from 'a to 'b where both are 'info)
                let transmuted_ptr: *const AccountInfo = std::mem::transmute(ptr);
                // Read the AccountInfo through the transmuted pointer
                std::ptr::read(transmuted_ptr)
            };
            
            // Convert ProgramError to Anchor Error
            invoke_signed(
                &transfer_ix,
                &[treasury_ai, user_token_ai, friend_group_ai, token_program_ai],
                signer_seeds,
            ).map_err(|e| anchor_lang::error::Error::from(e))?;
        }
    }
    
    // Update batch status
    batch.status = BatchStatus::Executed;
    batch.executed_at = Some(clock.unix_timestamp);
    batch.settlements = settlements;
    batch.total_sol_amount = total_sol;
    batch.total_usdc_amount = total_usdc;
    
    Ok(())
}


use anchor_lang::prelude::*;
use crate::errors::*;
use crate::state::{BatchSettlement, SettlementEntry, BatchStatus, TokenType};

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
    
    // Validate treasury_sol matches friend_group
    require!(
        ctx.accounts.treasury_sol.key() == friend_group.treasury_sol,
        TreasuryError::InvalidFriendGroup
    );
    
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
        
        // Process SOL or USDC transfer via CPI to friend_groups program
        let transfer_sol = if entry.token_type == TokenType::Sol { entry.amount } else { 0 };
        let transfer_usdc = if entry.token_type == TokenType::Usdc { entry.amount } else { 0 };
        
        if transfer_sol > 0 || transfer_usdc > 0 {
            // Clone AccountInfos from remaining_accounts and transmute to unified lifetime
            let user_wallet_cloned = user_wallet.clone();
            let user_token_cloned = user_token_account.clone();
            
            // SAFETY: All AccountInfos come from the same Context<'info>, so lifetimes are actually the same
            // Rust's type system just can't prove it due to variance rules
            let user_wallet_ai = unsafe {
                let ptr = &user_wallet_cloned as *const AccountInfo;
                let transmuted_ptr: *const AccountInfo = std::mem::transmute(ptr);
                std::ptr::read(transmuted_ptr)
            };
            
            let user_token_ai = unsafe {
                let ptr = &user_token_cloned as *const AccountInfo;
                let transmuted_ptr: *const AccountInfo = std::mem::transmute(ptr);
                std::ptr::read(transmuted_ptr)
            };
            
            // Call friend_groups treasury_transfer via CPI
            let cpi_program = ctx.accounts.friend_groups_program.to_account_info();
            let cpi_accounts = friend_groups::cpi::accounts::TreasuryTransfer {
                friend_group: ctx.accounts.friend_group.to_account_info(),
                treasury_sol: ctx.accounts.treasury_sol.to_account_info(),
                treasury_usdc: ctx.accounts.treasury_usdc.to_account_info(),
                destination_wallet: user_wallet_ai,
                destination_token_account: user_token_ai,
                token_program: ctx.accounts.token_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            friend_groups::cpi::treasury_transfer(cpi_ctx, transfer_sol, transfer_usdc)?;
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


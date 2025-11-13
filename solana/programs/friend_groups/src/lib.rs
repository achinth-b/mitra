use anchor_lang::prelude::*;

pub mod state; 
pub mod instructions;
pub mod errors;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod friend_groups {
    use super::*;

    pub fn create_group(
        ctx: Context<CreateGroup>, 
        name: String) -> Result<()> {
        instructions::create_group::handler(ctx, name)
    }

    pub fn invite_member(
        ctx: Context<InviteMember>,
    ) -> Result<()> {
        instructions::invite_member::handler(ctx)
    }

    pub fn accept_invite(
        ctx: Context<AcceptInvite>,
    ) -> Result<()> {
        instructions::accept_invite::handler(ctx)
    }

    pub fn remove_member(
        ctx: Context<RemoveMember>,
    ) -> Result<()> {
        instructions::remove_member::handler(ctx)
    }

    pub fn deposit_funds(
        ctx: Context<DepositFunds>,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> Result<()> {
        instructions::deposit_funds::handler(ctx, amount_sol, amount_usdc)
    }

    pub fn withdraw_funds(
        ctx: Context<WithdrawFunds>,
        amount_sol: u64,
        amount_usdc: u64,
    ) -> Result<()> {
        instructions::withdraw_funds::handler(ctx, amount_sol, amount_usdc)
    }
}
use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::*;

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

pub fn handler(ctx: Context<InviteMember>) -> Result<()> {
    let friend_group = &ctx.accounts.friend_group;
    let clock = Clock::get()?;
    
    // Only admin can invite (for now - can extend to members later)
    require!(
        friend_group.admin == ctx.accounts.inviter.key(),
        FriendGroupError::Unauthorized
    );
    
    // Check member limit
    require!(
        friend_group.member_count < FriendGroup::MAX_MEMBERS,
        FriendGroupError::MaxMembersReached
    );
    
    // Can't invite yourself
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


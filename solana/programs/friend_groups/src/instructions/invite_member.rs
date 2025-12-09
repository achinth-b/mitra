use anchor_lang::prelude::*;
use crate::errors::*;
use crate::state::{FriendGroup, Invite};

pub fn handler(ctx: Context<crate::InviteMember>) -> Result<()> {
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


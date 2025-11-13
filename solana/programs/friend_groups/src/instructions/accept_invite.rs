use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::*;

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

pub fn handler(ctx: Context<AcceptInvite>) -> Result<()> {
    let invite = &ctx.accounts.invite;
    let clock = Clock::get()?;
    
    // Validate invite hasn't expired
    require!(
        clock.unix_timestamp < invite.expires_at,
        FriendGroupError::InviteExpired
    );
    
    // Validate the signer is the invited user
    require!(
        ctx.accounts.invited_user.key() == invite.invited_user,
        FriendGroupError::Unauthorized
    );
    
    let friend_group = &mut ctx.accounts.friend_group;
    
    // Check member limit again (in case it changed since invite was created)
    require!(
        friend_group.member_count < FriendGroup::MAX_MEMBERS,
        FriendGroupError::MaxMembersReached
    );
    
    // Check if user is already a member (prevent duplicate memberships)
    // This is implicitly checked by the init constraint (account must not exist),
    // but we add explicit check for better error message
    // Note: The init constraint will fail if account exists, so this is defensive
    
    // Initialize group member
    let group_member = &mut ctx.accounts.group_member;
    group_member.user = ctx.accounts.invited_user.key();
    group_member.group = friend_group.key();
    group_member.role = MemberRole::Member;
    group_member.balance_sol = 0;
    group_member.balance_usdc = 0;
    group_member.locked_funds = false;
    group_member.joined_at = clock.unix_timestamp;
    
    // Increment member count
    friend_group.member_count = friend_group.member_count
        .checked_add(1)
        .ok_or(FriendGroupError::MaxMembersReached)?;
    
    Ok(())
}


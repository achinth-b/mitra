import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { FriendGroups } from "../target/types/friend_groups";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
} from "@solana/spl-token";
import { expect } from "chai";
import { FriendGroupTestHarness } from "./harness";

describe("Friend Groups", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  describe("create_group", () => {
    it("Successfully creates a friend group", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init("Test Group");

      // Verify group was created
      const groupAccount = await harness.getGroup();
      expect(groupAccount.name).to.equal("Test Group");
      expect(groupAccount.admin.toString()).to.equal(harness.admin.publicKey.toString());
      expect(groupAccount.memberCount).to.equal(1);
      expect(groupAccount.treasurySol.toString()).to.equal(harness.treasurySolPda.toString());
      expect(groupAccount.treasuryUsdc.toString()).to.equal(harness.treasuryUsdcPda.toString());
    });

    it("Fails with name too long", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      const longName = "a".repeat(51);

      await harness.expectError(async () => {
        await harness.init(longName);
      });
    });
  });

  describe("invite_member", () => {
    it("Admin successfully invites a member", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.createMember();

      // Create invite only (don't accept)
      await program.methods
        .inviteMember()
        .accounts({
          friendGroup: harness.friendGroupPda,
          invitedUser: member1.publicKey,
          inviter: harness.admin.publicKey,
        })
        .signers([harness.admin])
        .rpc();

      // Verify invite was created
      const invitePda = harness.getInvitePda(member1);
      const inviteAccount = await program.account.invite.fetch(invitePda);
      expect(inviteAccount.group.toString()).to.equal(harness.friendGroupPda.toString());
      expect(inviteAccount.invitedUser.toString()).to.equal(member1.publicKey.toString());
      expect(inviteAccount.inviter.toString()).to.equal(harness.admin.publicKey.toString());
      expect(inviteAccount.expiresAt.toNumber()).to.be.greaterThan(inviteAccount.createdAt.toNumber());
    });

    it("Fails when non-admin tries to invite", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const member2 = await harness.createMember();

      await harness.expectUnauthorizedError(async () => {
        await program.methods
          .inviteMember()
          .accounts({
            friendGroup: harness.friendGroupPda,
            invitedUser: member2.publicKey,
            inviter: member1.publicKey, // Not admin
          })
          .signers([member1])
          .rpc();
      });
    });

    it("Fails when trying to invite self", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();

      await harness.expectInvalidAmountError(async () => {
        await program.methods
          .inviteMember()
          .accounts({
            friendGroup: harness.friendGroupPda,
            invitedUser: harness.admin.publicKey,
            inviter: harness.admin.publicKey,
          })
          .signers([harness.admin])
          .rpc();
      });
    });
  });

  describe("accept_invite", () => {
    it("Successfully accepts an invite", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.createMember();

      // Create invite
      const invitePda = harness.getInvitePda(member1);
      await program.methods
        .inviteMember()
        .accounts({
          friendGroup: harness.friendGroupPda,
          invitedUser: member1.publicKey,
          inviter: harness.admin.publicKey,
        })
        .signers([harness.admin])
        .rpc();

      // Accept invite
      await program.methods
        .acceptInvite()
        .accounts({
          friendGroup: harness.friendGroupPda,
          invitedUser: member1.publicKey,
        })
        .signers([member1])
        .rpc();

      // Verify member was created
      const memberPda = harness.getMemberPda(member1);
      const memberAccount = await program.account.groupMember.fetch(memberPda);
      expect(memberAccount.user.toString()).to.equal(member1.publicKey.toString());
      expect(memberAccount.group.toString()).to.equal(harness.friendGroupPda.toString());
      expect(memberAccount.balanceSol.toNumber()).to.equal(0);
      expect(memberAccount.balanceUsdc.toNumber()).to.equal(0);
      expect(memberAccount.lockedFunds).to.be.false;

      // Verify invite was closed (consumed during accept)
      try {
        await program.account.invite.fetch(invitePda);
        expect.fail("Invite should have been closed");
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("does not exist");
      }

      // Verify member count increased
      const groupAccount = await harness.getGroup();
      expect(groupAccount.memberCount).to.equal(2);
    });

    it("Fails when wrong user tries to accept invite", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member2 = await harness.createMember();
      const member3 = await harness.createMember();

      // Create invite for member2
      await program.methods
        .inviteMember()
        .accounts({
          friendGroup: harness.friendGroupPda,
          invitedUser: member2.publicKey,
          inviter: harness.admin.publicKey,
        })
        .signers([harness.admin])
        .rpc();

      // Try to accept with wrong user
      await harness.expectUnauthorizedError(async () => {
        await program.methods
          .acceptInvite()
          .accounts({
            friendGroup: harness.friendGroupPda,
            invitedUser: member3.publicKey, // Wrong user
          })
          .signers([member3])
          .rpc();
      });
    });
  });

  describe("deposit_funds", () => {
    it("Successfully deposits SOL", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const depositAmount = 1 * LAMPORTS_PER_SOL;

      const treasuryBalanceBefore = await harness.getTreasurySolBalance();

      await harness.depositFor(member1, depositAmount, 0);

      // Verify member balance increased
      const memberAccount = await harness.getMember(member1);
      expect(memberAccount.balanceSol.toNumber()).to.equal(depositAmount);

      // Verify treasury received SOL
      const treasuryBalanceAfter = await harness.getTreasurySolBalance();
      expect(treasuryBalanceAfter - treasuryBalanceBefore).to.equal(depositAmount);
    });

    it("Successfully deposits USDC", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const depositAmount = 100 * 1e6; // 100 USDC

      const memberAccountBefore = await harness.getMember(member1);
      const treasuryUsdcBefore = await harness.getTreasuryUsdcBalance();

      await harness.depositFor(member1, 0, depositAmount);

      // Verify member balance increased
      const memberAccount = await harness.getMember(member1);
      expect(memberAccount.balanceUsdc.toNumber()).to.equal(
        memberAccountBefore.balanceUsdc.toNumber() + depositAmount
      );

      // Verify treasury received USDC
      const treasuryUsdcAfter = await harness.getTreasuryUsdcBalance();
      expect(treasuryUsdcAfter).to.equal(treasuryUsdcBefore + depositAmount);
    });

    it("Fails when non-member tries to deposit", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const nonMember = await harness.createMember();

      const nonMemberUsdcAccount = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        harness.usdcMint,
        nonMember.publicKey
      );

      await harness.expectUnauthorizedError(async () => {
        await program.methods
          .depositFunds(new anchor.BN(LAMPORTS_PER_SOL), new anchor.BN(0))
          .accounts({
            friendGroup: harness.friendGroupPda,
            memberWallet: nonMember.publicKey,
            treasuryUsdc: harness.treasuryUsdcPda,
            memberUsdcAccount: nonMemberUsdcAccount,
          })
          .signers([nonMember])
          .rpc();
      });
    });

    it("Fails when both amounts are zero", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const memberUsdcAccount = await harness.ensureTokenAccount(member1);

      await harness.expectInvalidAmountError(async () => {
        await program.methods
          .depositFunds(new anchor.BN(0), new anchor.BN(0))
          .accounts({
            friendGroup: harness.friendGroupPda,
            memberWallet: member1.publicKey,
            treasuryUsdc: harness.treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
          })
          .signers([member1])
          .rpc();
      });
    });
  });

  describe("withdraw_funds", () => {
    it("Successfully withdraws SOL", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const withdrawAmount = 0.5 * LAMPORTS_PER_SOL;

      // Deposit first to ensure balance
      await harness.depositFor(member1, withdrawAmount, 0);

      const memberAccountBefore = await harness.getMember(member1);
      const treasuryBalanceBefore = await harness.getTreasurySolBalance();

      await harness.withdrawFor(member1, withdrawAmount, 0);

      // Verify member balance decreased
      const memberAccount = await harness.getMember(member1);
      expect(memberAccount.balanceSol.toNumber()).to.be.lessThan(
        memberAccountBefore.balanceSol.toNumber()
      );

      // Verify treasury balance decreased
      const treasuryBalanceAfter = await harness.getTreasurySolBalance();
      expect(treasuryBalanceBefore - treasuryBalanceAfter).to.equal(withdrawAmount);
    });

    it("Successfully withdraws USDC", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const withdrawAmount = 50 * 1e6; // 50 USDC

      // Deposit first to ensure balance
      await harness.depositFor(member1, 0, withdrawAmount);

      const memberAccountBefore = await harness.getMember(member1);
      const treasuryUsdcBefore = await harness.getTreasuryUsdcBalance();

      await harness.withdrawFor(member1, 0, withdrawAmount);

      // Verify member balance decreased
      const memberAccount = await harness.getMember(member1);
      expect(memberAccount.balanceUsdc.toNumber()).to.equal(
        memberAccountBefore.balanceUsdc.toNumber() - withdrawAmount
      );

      // Verify treasury balance decreased
      const treasuryUsdcAfter = await harness.getTreasuryUsdcBalance();
      expect(treasuryUsdcAfter).to.equal(treasuryUsdcBefore - withdrawAmount);
    });

    it("Fails when withdrawing more than balance", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();

      const memberAccount = await harness.getMember(member1);
      const excessiveAmount = memberAccount.balanceSol.toNumber() + LAMPORTS_PER_SOL;
      const memberUsdcAccount = await harness.ensureTokenAccount(member1);

      await harness.expectError(async () => {
        await program.methods
          .withdrawFunds(new anchor.BN(excessiveAmount), new anchor.BN(0))
          .accounts({
            friendGroup: harness.friendGroupPda,
            memberWallet: member1.publicKey,
            treasuryUsdc: harness.treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
          })
          .signers([member1])
          .rpc();
      });
    });
  });

  describe("remove_member", () => {
    it("Successfully removes a member and refunds balances", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      
      // Add 4 members to ensure we can remove one
      await harness.addMember();
      const member2 = await harness.addMember();
      await harness.addMember();
      await harness.addMember();

      // Deposit some balance for member2
      await harness.depositFor(member2, LAMPORTS_PER_SOL, 0);

      const groupAccountBefore = await harness.getGroup();
      const memberPda = harness.getMemberPda(member2);

      await harness.removeMember(member2);

      // Verify member count decreased
      const groupAccount = await harness.getGroup();
      expect(groupAccount.memberCount).to.equal(groupAccountBefore.memberCount - 1);

      // Verify member account was closed
      try {
        await program.account.groupMember.fetch(memberPda);
        expect.fail("Member account should have been closed");
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("does not exist") || expect(errorMsg).to.include("AccountNotFound");
      }
    });

    it("Fails when non-admin tries to remove member", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      const member1 = await harness.addMember();
      const member2 = await harness.addMember();

      const memberUsdcAccount = await harness.ensureTokenAccount(member2);

      await harness.expectUnauthorizedError(async () => {
        await program.methods
          .removeMember()
          .accounts({
            friendGroup: harness.friendGroupPda,
            memberWallet: member2.publicKey,
            treasuryUsdc: harness.treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
            admin: member1.publicKey, // Not admin
          })
          .signers([member1])
          .rpc();
      });
    });

    it("Fails when removing would violate minimum member requirement", async () => {
      const harness = new FriendGroupTestHarness(program, provider);
      await harness.init();
      
      // Add exactly 2 more members (total 3 including admin)
      const member1 = await harness.addMember();
      await harness.addMember();

      const groupAccount = await harness.getGroup();
      expect(groupAccount.memberCount).to.equal(3);

      const memberUsdcAccount = await harness.ensureTokenAccount(member1);

      await harness.expectMinMembersError(async () => {
        await program.methods
          .removeMember()
          .accounts({
            friendGroup: harness.friendGroupPda,
            memberWallet: member1.publicKey,
            treasuryUsdc: harness.treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
            admin: harness.admin.publicKey,
          })
          .signers([harness.admin])
          .rpc();
      });
    });
  });
});

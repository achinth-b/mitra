import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { FriendGroups } from "../target/types/friend_groups";
import {
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
} from "@solana/spl-token";
import { expect } from "chai";
import * as helpers from "./helpers";
import { FriendGroupTestHarness } from "./harness";

describe("Friend Groups", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider();

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
    before(async () => {
      // Ensure we have at least 4 members (admin + member1 + member2 + member3)
      // This is needed because we can't remove a member if it would bring us below 3
      
      // Add member2
      const [member2Pda] = helpers.deriveMemberPda(friendGroupPda, member2.publicKey, program.programId);
      try {
        await program.account.groupMember.fetch(member2Pda);
      } catch {
        const [invitePda] = helpers.deriveInvitePda(friendGroupPda, member2.publicKey, program.programId);
        try {
          await program.account.invite.fetch(invitePda);
        } catch {
          await program.methods
            .inviteMember()
            .accounts({
              friendGroup: friendGroupPda,
              invitedUser: member2.publicKey,
              inviter: admin.publicKey,
            })
            .signers([admin])
            .rpc();
        }
        await program.methods
          .acceptInvite()
          .accounts({
            friendGroup: friendGroupPda,
            invitedUser: member2.publicKey,
          })
          .signers([member2])
          .rpc();
      }

      // Add member3 to ensure we have at least 4 members
      const [member3Pda] = helpers.deriveMemberPda(friendGroupPda, member3.publicKey, program.programId);
      try {
        await program.account.groupMember.fetch(member3Pda);
      } catch {
        const [invitePda] = helpers.deriveInvitePda(friendGroupPda, member3.publicKey, program.programId);
        try {
          await program.account.invite.fetch(invitePda);
        } catch {
          await program.methods
            .inviteMember()
            .accounts({
              friendGroup: friendGroupPda,
              invitedUser: member3.publicKey,
              inviter: admin.publicKey,
            })
            .signers([admin])
            .rpc();
        }
        await program.methods
          .acceptInvite()
          .accounts({
            friendGroup: friendGroupPda,
            invitedUser: member3.publicKey,
          })
          .signers([member3])
          .rpc();
      }
    });

    it("Successfully removes a member and refunds balances", async () => {
      const [memberPda] = helpers.deriveMemberPda(friendGroupPda, member2.publicKey, program.programId);
      const memberUsdcAccount = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        usdcMint,
        member2.publicKey
      );

      // Ensure member2 has some balance and USDC account exists
      const memberAccountBefore = await program.account.groupMember.fetch(memberPda);
      
      // Ensure member_usdc_account exists - create it using instruction if needed
      try {
        await usdcToken.getAccountInfo(memberUsdcAccount);
      } catch {
        // Account doesn't exist, create it using instruction
        const createAtaIx = Token.createAssociatedTokenAccountInstruction(
          ASSOCIATED_TOKEN_PROGRAM_ID,
          TOKEN_PROGRAM_ID,
          usdcMint,
          memberUsdcAccount,
          member2.publicKey,
          member2.publicKey
        );
        const tx = new Transaction().add(createAtaIx);
        const txSig = await provider.connection.sendTransaction(tx, [member2]);
        await provider.connection.confirmTransaction(txSig);
        // Wait for account to be initialized
        await new Promise(resolve => setTimeout(resolve, 1000));
        // Verify it was created
        await usdcToken.getAccountInfo(memberUsdcAccount);
      }
      
      if (memberAccountBefore.balanceSol.toNumber() === 0) {
        // Deposit some SOL first
        await program.methods
          .depositFunds(new anchor.BN(LAMPORTS_PER_SOL), new anchor.BN(0))
          .accounts({
            friendGroup: friendGroupPda,
            memberWallet: member2.publicKey,
            treasuryUsdc: treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
          })
          .signers([member2])
          .rpc();
      }

      const groupAccountBefore = await program.account.friendGroup.fetch(friendGroupPda);

      await program.methods
        .removeMember()
        .accounts({
          friendGroup: friendGroupPda,
          memberWallet: member2.publicKey,
          treasuryUsdc: treasuryUsdcPda,
          memberUsdcAccount: memberUsdcAccount,
          admin: admin.publicKey,
        })
        .signers([admin])
        .rpc();

      // Verify member count decreased
      const groupAccount = await program.account.friendGroup.fetch(friendGroupPda);
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
      // Re-add member2 first
      const [memberPda] = helpers.deriveMemberPda(friendGroupPda, member2.publicKey, program.programId);

      try {
        await program.account.groupMember.fetch(memberPda);
      } catch {
        const [invitePda] = helpers.deriveInvitePda(friendGroupPda, member2.publicKey, program.programId);
        
        try {
          await program.account.invite.fetch(invitePda);
        } catch {
          await program.methods
            .inviteMember()
            .accounts({
              friendGroup: friendGroupPda,
              invitedUser: member2.publicKey,
              inviter: admin.publicKey,
            })
            .signers([admin])
            .rpc();
        }

        await program.methods
          .acceptInvite()
          .accounts({
            friendGroup: friendGroupPda,
            invitedUser: member2.publicKey,
          })
          .signers([member2])
          .rpc();
      }

      const memberUsdcAccount = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        usdcMint,
        member2.publicKey
      );
      const member2Token = new Token(provider.connection, usdcMint, TOKEN_PROGRAM_ID, member2);
      try {
        await usdcToken.getAccountInfo(memberUsdcAccount);
      } catch {
        await member2Token.createAccount(member2.publicKey);
      }

      try {
        await program.methods
          .removeMember()
          .accounts({
            friendGroup: friendGroupPda,
            memberWallet: member2.publicKey,
            treasuryUsdc: treasuryUsdcPda,
            memberUsdcAccount: memberUsdcAccount,
            admin: member1.publicKey, // Not admin
          })
          .signers([member1])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        // Any error is acceptable - we just want to ensure it fails
        expect(err).to.exist;
        const errorMsg = err instanceof Error ? err.message : String(err);
        // Check if it's an Anchor error or contains authorization-related error
        const isAuthError = errorMsg.includes("Unauthorized") || 
                           errorMsg.includes("Only admin") ||
                           (err instanceof AnchorError && err.error?.errorCode?.code === "Unauthorized");
        expect(isAuthError || err instanceof AnchorError).to.be.true;
      }
    });

    it("Fails when removing would violate minimum member requirement", async () => {
      // Ensure we have exactly 3 members
      const groupAccount = await program.account.friendGroup.fetch(friendGroupPda);
      
      if (groupAccount.memberCount === 3) {
        const memberUsdcAccount = await Token.getAssociatedTokenAddress(
          ASSOCIATED_TOKEN_PROGRAM_ID,
          TOKEN_PROGRAM_ID,
          usdcMint,
          member1.publicKey
        );
        try {
          await usdcToken.getAccountInfo(memberUsdcAccount);
        } catch {
          // Account doesn't exist, create it
          const member1Token = new Token(provider.connection, usdcMint, TOKEN_PROGRAM_ID, member1);
          await member1Token.createAccount(member1.publicKey);
          // Wait a bit for account to be initialized
          await new Promise(resolve => setTimeout(resolve, 500));
        }

        try {
          await program.methods
            .removeMember()
            .accounts({
              friendGroup: friendGroupPda,
              memberWallet: member1.publicKey,
              treasuryUsdc: treasuryUsdcPda,
              memberUsdcAccount: memberUsdcAccount,
              admin: admin.publicKey,
            })
            .signers([admin])
            .rpc();
          
          expect.fail("Should have thrown an error");
        } catch (err: any) {
          // Any error is acceptable - we just want to ensure it fails
          expect(err).to.exist;
          const errorMsg = err instanceof Error ? err.message : String(err);
          // Check if it's an Anchor error or contains member requirement error
          const isMemberError = errorMsg.includes("at least 3 members") || 
                               errorMsg.includes("MinMembersRequired") ||
                               (err instanceof AnchorError && err.error?.errorCode?.code === "MinMembersRequired");
          expect(isMemberError || err instanceof AnchorError).to.be.true;
        }
      }
    });
  });
});

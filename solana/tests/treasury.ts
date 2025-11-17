import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { Treasury } from "../target/types/treasury";
import { FriendGroups } from "../target/types/friend_groups";
import { LAMPORTS_PER_SOL, PublicKey, Keypair } from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { expect } from "chai";
import { FriendGroupTestHarness } from "./harness";
import * as helpers from "./helpers";

describe("Treasury", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const treasuryProgram = anchor.workspace.Treasury as Program<Treasury>;
  const friendGroupsProgram = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  describe("batch_settle", () => {
    it("Successfully creates and executes a batch settlement with SOL", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      const member1 = await harness.addMember();
      const member2 = await harness.addMember();
      
      // Deposit funds for members
      const depositAmount = 2 * LAMPORTS_PER_SOL;
      await harness.depositFor(member1, depositAmount, 0);
      await harness.depositFor(member2, depositAmount, 0);
      
      const batchId = 1;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      // Create settlement entries
      const settlement1 = {
        user: member1.publicKey,
        event: Keypair.generate().publicKey, // Mock event
        amount: new anchor.BN(LAMPORTS_PER_SOL),
        tokenType: { sol: {} },
      };
      
      const settlement2 = {
        user: member2.publicKey,
        event: Keypair.generate().publicKey, // Mock event
        amount: new anchor.BN(LAMPORTS_PER_SOL),
        tokenType: { sol: {} },
      };
      
      const settlements = [settlement1, settlement2];
      
      // Get user wallets and token accounts for remaining accounts
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      const member2TokenAccount = await harness.ensureTokenAccount(member2);
      
      const treasuryBalanceBefore = await harness.getTreasurySolBalance();
      const member1BalanceBefore = await provider.connection.getBalance(member1.publicKey);
      const member2BalanceBefore = await provider.connection.getBalance(member2.publicKey);
      
      await treasuryProgram.methods
        .batchSettle(
          new anchor.BN(batchId),
          settlements.map(s => ({
            user: s.user,
            event: s.event,
            amount: s.amount,
            tokenType: s.tokenType,
          }))
        )
        .accounts({
          friendGroup: harness.friendGroupPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
        })
        .remainingAccounts([
          { pubkey: member1.publicKey, isSigner: false, isWritable: true },
          { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          { pubkey: member2.publicKey, isSigner: false, isWritable: true },
          { pubkey: member2TokenAccount, isSigner: false, isWritable: true },
        ])
        .signers([harness.admin])
        .rpc();
      
      // Verify batch was created
      const batchAccount = await treasuryProgram.account.batchSettlement.fetch(batchPda);
      expect(batchAccount.batchId.toNumber()).to.equal(batchId);
      expect(batchAccount.status).to.deep.equal({ executed: {} });
      expect(batchAccount.totalSolAmount.toNumber()).to.equal(2 * LAMPORTS_PER_SOL);
      
      // Verify treasury balance decreased
      const treasuryBalanceAfter = await harness.getTreasurySolBalance();
      expect(treasuryBalanceBefore - treasuryBalanceAfter).to.equal(2 * LAMPORTS_PER_SOL);
      
      // Verify users received SOL (allowing for transaction fees)
      const member1BalanceAfter = await provider.connection.getBalance(member1.publicKey);
      const member2BalanceAfter = await provider.connection.getBalance(member2.publicKey);
      expect(member1BalanceAfter - member1BalanceBefore).to.be.greaterThan(LAMPORTS_PER_SOL * 0.99);
      expect(member2BalanceAfter - member2BalanceBefore).to.be.greaterThan(LAMPORTS_PER_SOL * 0.99);
    });

    it("Successfully creates and executes a batch settlement with USDC", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      const member1 = await harness.addMember();
      const member2 = await harness.addMember();
      
      // Deposit USDC for members
      const depositAmount = 100 * 1e6; // 100 USDC
      await harness.depositFor(member1, 0, depositAmount);
      await harness.depositFor(member2, 0, depositAmount);
      
      const batchId = 2;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      // Create settlement entries
      const settlement1 = {
        user: member1.publicKey,
        event: Keypair.generate().publicKey,
        amount: new anchor.BN(50 * 1e6), // 50 USDC
        tokenType: { usdc: {} },
      };
      
      const settlement2 = {
        user: member2.publicKey,
        event: Keypair.generate().publicKey,
        amount: new anchor.BN(50 * 1e6), // 50 USDC
        tokenType: { usdc: {} },
      };
      
      const settlements = [settlement1, settlement2];
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      const member2TokenAccount = await harness.ensureTokenAccount(member2);
      
      const treasuryUsdcBefore = await harness.getTreasuryUsdcBalance();
      
      await treasuryProgram.methods
        .batchSettle(
          new anchor.BN(batchId),
          settlements.map(s => ({
            user: s.user,
            event: s.event,
            amount: s.amount,
            tokenType: s.tokenType,
          }))
        )
        .accounts({
          batchSettlement: batchPda,
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
        })
        .remainingAccounts([
          { pubkey: member1.publicKey, isSigner: false, isWritable: true },
          { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          { pubkey: member2.publicKey, isSigner: false, isWritable: true },
          { pubkey: member2TokenAccount, isSigner: false, isWritable: true },
        ])
        .signers([harness.admin])
        .rpc();
      
      // Verify batch was executed
      const batchAccount = await treasuryProgram.account.batchSettlement.fetch(batchPda);
      expect(batchAccount.status).to.deep.equal({ executed: {} });
      expect(batchAccount.totalUsdcAmount.toNumber()).to.equal(100 * 1e6);
      
      // Verify treasury balance decreased
      const treasuryUsdcAfter = await harness.getTreasuryUsdcBalance();
      expect(treasuryUsdcBefore - treasuryUsdcAfter).to.equal(100 * 1e6);
    });

    it("Fails when non-admin tries to batch settle", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      const batchId = 3;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      const settlement = {
        user: member1.publicKey,
        event: Keypair.generate().publicKey,
        amount: new anchor.BN(LAMPORTS_PER_SOL),
        tokenType: { sol: {} },
      };
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      try {
        await treasuryProgram.methods
          .batchSettle(new anchor.BN(batchId), [settlement])
          .accounts({
            batchSettlement: batchPda,
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: member1.publicKey, // Not admin
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([member1])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        if (err instanceof AnchorError) {
          expect(err.error.errorCode.code).to.equal("Unauthorized");
        } else {
          const errorMsg = err instanceof Error ? err.message : String(err);
          expect(errorMsg).to.include("Unauthorized");
        }
      }
    });

    it("Fails with insufficient treasury balance", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      // Don't deposit enough funds
      await harness.depositFor(member1, 0.5 * LAMPORTS_PER_SOL, 0);
      
      const batchId = 5;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      const settlement = {
        user: member1.publicKey,
        event: Keypair.generate().publicKey,
        amount: new anchor.BN(LAMPORTS_PER_SOL), // More than deposited
        tokenType: { sol: {} },
      };
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      try {
        await treasuryProgram.methods
          .batchSettle(new anchor.BN(batchId), [settlement])
          .accounts({
            batchSettlement: batchPda,
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: harness.admin.publicKey,
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        if (err instanceof AnchorError) {
          expect(err.error.errorCode.code).to.equal("InsufficientBalance");
        } else {
          const errorMsg = err instanceof Error ? err.message : String(err);
          expect(errorMsg).to.include("insufficient") || expect(errorMsg).to.include("balance");
        }
      }
    });

    it("Fails when batch already executed", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      await harness.depositFor(member1, LAMPORTS_PER_SOL, 0);
      
      const batchId = 4;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      const settlement = {
        user: member1.publicKey,
        event: Keypair.generate().publicKey,
        amount: new anchor.BN(LAMPORTS_PER_SOL),
        tokenType: { sol: {} },
      };
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      // Execute first time
      await treasuryProgram.methods
        .batchSettle(new anchor.BN(batchId), [settlement])
        .accounts({
          batchSettlement: batchPda,
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
        })
        .remainingAccounts([
          { pubkey: member1.publicKey, isSigner: false, isWritable: true },
          { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
        ])
        .signers([harness.admin])
        .rpc();
      
      // Try to execute again
      try {
        await treasuryProgram.methods
          .batchSettle(new anchor.BN(batchId), [settlement])
          .accounts({
            batchSettlement: batchPda,
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: harness.admin.publicKey,
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        if (err instanceof AnchorError) {
          expect(err.error.errorCode.code).to.equal("BatchAlreadyExecuted");
        } else {
          const errorMsg = err instanceof Error ? err.message : String(err);
          expect(errorMsg).to.include("already executed");
        }
      }
    });
  });

  describe("emergency_withdraw", () => {
    it("Successfully creates an emergency withdrawal request", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      // Deposit some funds
      await harness.depositFor(harness.admin, LAMPORTS_PER_SOL, 100 * 1e6);
      
      const requestId = 1;
      const [withdrawPda] = deriveEmergencyWithdrawPda(
        harness.friendGroupPda,
        requestId,
        treasuryProgram.programId
      );
      
      const destination = Keypair.generate();
      await helpers.airdropSol(provider.connection, destination.publicKey, 0.1 * LAMPORTS_PER_SOL);
      
      const destinationTokenAccount = await getAssociatedTokenAddress(
        harness.usdcMint,
        destination.publicKey,
        true
      );
      
      await treasuryProgram.methods
        .emergencyWithdraw(
          new anchor.BN(requestId),
          new anchor.BN(LAMPORTS_PER_SOL),
          new anchor.BN(50 * 1e6)
        )
        .accounts({
          emergencyWithdraw: withdrawPda,
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          destination: destination.publicKey,
          destinationTokenAccount: destinationTokenAccount,
          admin: harness.admin.publicKey,
        })
        .signers([harness.admin])
        .rpc();
      
      // Verify request was created
      const withdrawAccount = await treasuryProgram.account.emergencyWithdraw.fetch(withdrawPda);
      expect(withdrawAccount.requestId.toNumber()).to.equal(requestId);
      expect(withdrawAccount.status).to.deep.equal({ pending: {} });
      expect(withdrawAccount.solAmount.toNumber()).to.equal(LAMPORTS_PER_SOL);
      expect(withdrawAccount.usdcAmount.toNumber()).to.equal(50 * 1e6);
      expect(withdrawAccount.unlockAt.toNumber()).to.be.greaterThan(withdrawAccount.requestedAt.toNumber());
    });

    it("Fails when non-admin tries to create emergency withdrawal", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      const requestId = 2;
      const [withdrawPda] = deriveEmergencyWithdrawPda(
        harness.friendGroupPda,
        requestId,
        treasuryProgram.programId
      );
      
      const destination = Keypair.generate();
      const destinationTokenAccount = Keypair.generate().publicKey;
      
      try {
        await treasuryProgram.methods
          .emergencyWithdraw(
            new anchor.BN(requestId),
            new anchor.BN(LAMPORTS_PER_SOL),
            new anchor.BN(0)
          )
          .accounts({
            emergencyWithdraw: withdrawPda,
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            destination: destination.publicKey,
            destinationTokenAccount: destinationTokenAccount,
            admin: member1.publicKey, // Not admin
          })
          .signers([member1])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        if (err instanceof AnchorError) {
          expect(err.error.errorCode.code).to.equal("Unauthorized");
        } else {
          const errorMsg = err instanceof Error ? err.message : String(err);
          expect(errorMsg).to.include("Unauthorized");
        }
      }
    });

    it("Fails when trying to execute before timelock expires", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      await harness.depositFor(harness.admin, LAMPORTS_PER_SOL, 0);
      
      const requestId = 3;
      const [withdrawPda] = deriveEmergencyWithdrawPda(
        harness.friendGroupPda,
        requestId,
        treasuryProgram.programId
      );
      
      const destination = Keypair.generate();
      await helpers.airdropSol(
        provider.connection,
        destination.publicKey,
        0.1 * LAMPORTS_PER_SOL
      );
      
      const destinationTokenAccount = await getAssociatedTokenAddress(
        harness.usdcMint,
        destination.publicKey,
        true
      );
      
      // Create request
      await treasuryProgram.methods
        .emergencyWithdraw(
          new anchor.BN(requestId),
          new anchor.BN(LAMPORTS_PER_SOL),
          new anchor.BN(0)
        )
        .accounts({
          emergencyWithdraw: withdrawPda,
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          destination: destination.publicKey,
          destinationTokenAccount: destinationTokenAccount,
          admin: harness.admin.publicKey,
        })
        .signers([harness.admin])
        .rpc();
      
      // Verify request was created with future unlock time
      const withdrawAccount = await treasuryProgram.account.emergencyWithdraw.fetch(withdrawPda);
      expect(withdrawAccount.unlockAt.toNumber()).to.be.greaterThan(
        withdrawAccount.requestedAt.toNumber()
      );
      
      // Try to execute immediately (should fail)
      try {
        await treasuryProgram.methods
          .emergencyWithdraw(
            new anchor.BN(requestId),
            new anchor.BN(LAMPORTS_PER_SOL),
            new anchor.BN(0)
          )
          .accounts({
            emergencyWithdraw: withdrawPda,
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            destination: destination.publicKey,
            destinationTokenAccount: destinationTokenAccount,
            admin: harness.admin.publicKey,
          })
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        if (err instanceof AnchorError) {
          expect(err.error.errorCode.code).to.equal("TimelockNotExpired");
        } else {
          const errorMsg = err instanceof Error ? err.message : String(err);
          expect(errorMsg).to.include("timelock");
        }
      }
    });
  });
});

// Helper functions
function deriveBatchSettlementPda(
  friendGroup: PublicKey,
  batchId: number,
  programId: PublicKey
): [PublicKey, number] {
  const buffer = Buffer.allocUnsafe(8);
  buffer.writeBigUInt64LE(BigInt(batchId), 0);
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("batch_settlement"),
      friendGroup.toBuffer(),
      buffer,
    ],
    programId
  );
}

function deriveEmergencyWithdrawPda(
  friendGroup: PublicKey,
  requestId: number,
  programId: PublicKey
): [PublicKey, number] {
  const buffer = Buffer.allocUnsafe(8);
  buffer.writeBigUInt64LE(BigInt(requestId), 0);
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("emergency_withdraw"),
      friendGroup.toBuffer(),
      buffer,
    ],
    programId
  );
}


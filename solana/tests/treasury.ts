import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Treasury } from "../target/types/treasury";
import { FriendGroups } from "../target/types/friend_groups";
import { LAMPORTS_PER_SOL, PublicKey, Keypair, Transaction } from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { expect } from "chai";
import { FriendGroupTestHarness } from "./harness";
import * as helpers from "./helpers";

// Test constants
const TEST_CONSTANTS = {
  SOL_DEPOSIT_AMOUNT: 2 * LAMPORTS_PER_SOL,
  SOL_WITHDRAWAL_AMOUNT: 0.5 * LAMPORTS_PER_SOL,
  SOL_SETTLEMENT_AMOUNT: LAMPORTS_PER_SOL,
  USDC_DEPOSIT_AMOUNT: 100 * 1e6, // 100 USDC
  USDC_SETTLEMENT_AMOUNT: 50 * 1e6, // 50 USDC
  USDC_TOLERANCE: 0.99, // Allow 1% tolerance for transaction fees
  TIMELOCK_BUFFER: 0.1 * LAMPORTS_PER_SOL,
} as const;

describe("Treasury", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const treasuryProgram = anchor.workspace.Treasury as Program<Treasury>;
  const friendGroupsProgram = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  // Helper functions
  async function createSettlement(user: Keypair, amount: anchor.BN, tokenType: any) {
    return {
      user: user.publicKey,
      event: Keypair.generate().publicKey,
      amount,
      tokenType,
    };
  }

  async function setupDestinationAccount(harness: FriendGroupTestHarness, destination: Keypair) {
    await helpers.airdropSol(
      provider.connection,
      destination.publicKey,
      TEST_CONSTANTS.TIMELOCK_BUFFER
    );

    const destinationTokenAccount = await getAssociatedTokenAddress(
      harness.usdcMint,
      destination.publicKey,
      true
    );

    const createAtaIx = createAssociatedTokenAccountInstruction(
      harness.admin.publicKey,
      destinationTokenAccount,
      destination.publicKey,
      harness.usdcMint
    );
    const tx = new Transaction().add(createAtaIx);
    await provider.connection.sendTransaction(tx, [harness.admin]);
    await new Promise((resolve) => setTimeout(resolve, 1000));

    return destinationTokenAccount;
  }

  describe("batch_settle", () => {
    it("Successfully creates and executes a batch settlement with SOL", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      const member1 = await harness.addMember();
      const member2 = await harness.addMember();
      
      // Deposit funds for members
      await harness.depositFor(member1, TEST_CONSTANTS.SOL_DEPOSIT_AMOUNT, 0);
      await harness.depositFor(member2, TEST_CONSTANTS.SOL_DEPOSIT_AMOUNT, 0);
      
      const batchId = 1;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      // Create settlement entries
      const settlement1 = await createSettlement(
        member1,
        new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
        { sol: {} }
      );
      
      const settlement2 = await createSettlement(
        member2,
        new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
        { sol: {} }
      );
      
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
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
          friendGroupsProgram: friendGroupsProgram.programId,
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
      expect(batchAccount.totalSolAmount.toNumber()).to.equal(TEST_CONSTANTS.SOL_DEPOSIT_AMOUNT);
      
      // Verify treasury balance decreased
      const treasuryBalanceAfter = await harness.getTreasurySolBalance();
      expect(treasuryBalanceBefore - treasuryBalanceAfter).to.equal(TEST_CONSTANTS.SOL_DEPOSIT_AMOUNT);
      
      // Verify users received SOL (allowing for transaction fees)
      const member1BalanceAfter = await provider.connection.getBalance(member1.publicKey);
      const member2BalanceAfter = await provider.connection.getBalance(member2.publicKey);
      expect(member1BalanceAfter - member1BalanceBefore).to.be.greaterThan(
        TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT * TEST_CONSTANTS.USDC_TOLERANCE
      );
      expect(member2BalanceAfter - member2BalanceBefore).to.be.greaterThan(
        TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT * TEST_CONSTANTS.USDC_TOLERANCE
      );
    });

    it("Successfully creates and executes a batch settlement with USDC", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      const member1 = await harness.addMember();
      const member2 = await harness.addMember();
      
      // Deposit USDC for members
      await harness.depositFor(member1, 0, TEST_CONSTANTS.USDC_DEPOSIT_AMOUNT);
      await harness.depositFor(member2, 0, TEST_CONSTANTS.USDC_DEPOSIT_AMOUNT);
      
      const batchId = 2;
      const [batchPda] = deriveBatchSettlementPda(
        harness.friendGroupPda,
        batchId,
        treasuryProgram.programId
      );
      
      // Create settlement entries
      const settlement1 = await createSettlement(
        member1,
        new anchor.BN(TEST_CONSTANTS.USDC_SETTLEMENT_AMOUNT),
        { usdc: {} }
      );
      
      const settlement2 = await createSettlement(
        member2,
        new anchor.BN(TEST_CONSTANTS.USDC_SETTLEMENT_AMOUNT),
        { usdc: {} }
      );
      
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
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
          friendGroupsProgram: friendGroupsProgram.programId,
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
      expect(batchAccount.totalUsdcAmount.toNumber()).to.equal(TEST_CONSTANTS.USDC_DEPOSIT_AMOUNT);
      
      // Verify treasury balance decreased
      const treasuryUsdcAfter = await harness.getTreasuryUsdcBalance();
      expect(treasuryUsdcBefore - treasuryUsdcAfter).to.equal(TEST_CONSTANTS.USDC_DEPOSIT_AMOUNT);
    });

    it("Fails when non-admin tries to batch settle", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      const batchId = 3;
      
      const settlement = await createSettlement(
        member1,
        new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
        { sol: {} }
      );
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      try {
        await treasuryProgram.methods
          .batchSettle(new anchor.BN(batchId), [settlement])
          .accounts({
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: member1.publicKey, // Not admin
            friendGroupsProgram: friendGroupsProgram.programId,
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([member1])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "Unauthorized");
      }
    });

    it("Fails with insufficient treasury balance", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      // Don't deposit enough funds
      await harness.depositFor(member1, TEST_CONSTANTS.SOL_WITHDRAWAL_AMOUNT, 0);
      
      const batchId = 5;
      
      const settlement = await createSettlement(
        member1,
        new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
        { sol: {} }
      );
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      try {
        await treasuryProgram.methods
          .batchSettle(new anchor.BN(batchId), [settlement])
          .accounts({
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: harness.admin.publicKey,
            friendGroupsProgram: friendGroupsProgram.programId,
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "InsufficientBalance");
      }
    });

    it("Fails when batch already executed", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      await harness.depositFor(member1, TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT, 0);
      
      const batchId = 4;
      
      const settlement = await createSettlement(
        member1,
        new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
        { sol: {} }
      );
      
      const member1TokenAccount = await harness.ensureTokenAccount(member1);
      
      // Execute first time
      await treasuryProgram.methods
        .batchSettle(new anchor.BN(batchId), [settlement])
        .accounts({
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          admin: harness.admin.publicKey,
          friendGroupsProgram: friendGroupsProgram.programId,
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
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            admin: harness.admin.publicKey,
            friendGroupsProgram: friendGroupsProgram.programId,
          })
          .remainingAccounts([
            { pubkey: member1.publicKey, isSigner: false, isWritable: true },
            { pubkey: member1TokenAccount, isSigner: false, isWritable: true },
          ])
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "BatchAlreadyExecuted");
      }
    });
  });

  describe("emergency_withdraw", () => {
    it("Successfully creates an emergency withdrawal request", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      // Add a member and deposit funds
      const member1 = await harness.addMember();
      await harness.depositFor(
        member1,
        TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT,
        TEST_CONSTANTS.USDC_DEPOSIT_AMOUNT
      );
      
      const requestId = 1;
      const [withdrawPda] = deriveEmergencyWithdrawPda(
        harness.friendGroupPda,
        requestId,
        treasuryProgram.programId
      );
      
      const destination = Keypair.generate();
      const destinationTokenAccount = await setupDestinationAccount(harness, destination);
      
      await treasuryProgram.methods
        .emergencyWithdraw(
          new anchor.BN(requestId),
          new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
          new anchor.BN(TEST_CONSTANTS.USDC_SETTLEMENT_AMOUNT)
        )
        .accounts({
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          destination: destination.publicKey,
          destinationTokenAccount: destinationTokenAccount,
          admin: harness.admin.publicKey,
          friendGroupsProgram: friendGroupsProgram.programId,
        })
        .signers([harness.admin])
        .rpc();
      
      // Verify request was created
      const withdrawAccount = await treasuryProgram.account.emergencyWithdraw.fetch(withdrawPda);
      expect(withdrawAccount.requestId.toNumber()).to.equal(requestId);
      expect(withdrawAccount.status).to.deep.equal({ pending: {} });
      expect(withdrawAccount.solAmount.toNumber()).to.equal(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT);
      expect(withdrawAccount.usdcAmount.toNumber()).to.equal(TEST_CONSTANTS.USDC_SETTLEMENT_AMOUNT);
      expect(withdrawAccount.unlockAt.toNumber()).to.be.greaterThan(
        withdrawAccount.requestedAt.toNumber()
      );
    });

    it("Fails when non-admin tries to create emergency withdrawal", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      const member1 = await harness.addMember();
      
      const requestId = 2;
      
      const destination = Keypair.generate();
      const destinationTokenAccount = await setupDestinationAccount(harness, destination);
      
      try {
        await treasuryProgram.methods
          .emergencyWithdraw(
            new anchor.BN(requestId),
            new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
            new anchor.BN(0)
          )
          .accounts({
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            destination: destination.publicKey,
            destinationTokenAccount: destinationTokenAccount,
            admin: member1.publicKey, // Not admin
            friendGroupsProgram: friendGroupsProgram.programId,
          })
          .signers([member1])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "Unauthorized");
      }
    });

    it("Fails when trying to execute before timelock expires", async () => {
      const harness = new FriendGroupTestHarness(friendGroupsProgram, provider);
      await harness.init("Test Group");
      
      // Add a member and deposit funds
      const member1 = await harness.addMember();
      await harness.depositFor(member1, TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT, 0);
      
      const requestId = 3;
      const [withdrawPda] = deriveEmergencyWithdrawPda(
        harness.friendGroupPda,
        requestId,
        treasuryProgram.programId
      );
      
      const destination = Keypair.generate();
      const destinationTokenAccount = await setupDestinationAccount(harness, destination);
      
      // Create request
      await treasuryProgram.methods
        .emergencyWithdraw(
          new anchor.BN(requestId),
          new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
          new anchor.BN(0)
        )
        .accounts({
          friendGroup: harness.friendGroupPda,
          treasurySol: harness.treasurySolPda,
          treasuryUsdc: harness.treasuryUsdcPda,
          destination: destination.publicKey,
          destinationTokenAccount: destinationTokenAccount,
          admin: harness.admin.publicKey,
          friendGroupsProgram: friendGroupsProgram.programId,
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
            new anchor.BN(TEST_CONSTANTS.SOL_SETTLEMENT_AMOUNT),
            new anchor.BN(0)
          )
          .accounts({
            friendGroup: harness.friendGroupPda,
            treasurySol: harness.treasurySolPda,
            treasuryUsdc: harness.treasuryUsdcPda,
            destination: destination.publicKey,
            destinationTokenAccount: destinationTokenAccount,
            admin: harness.admin.publicKey,
            friendGroupsProgram: friendGroupsProgram.programId,
          })
          .signers([harness.admin])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "TimelockNotExpired");
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


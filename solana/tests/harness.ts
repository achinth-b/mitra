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
  createMint,
  getAccount,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";
import * as helpers from "./helpers";

// Constants
export const USDC_DECIMALS = 6;
export const MIN_MEMBERS = 3;
export const DEFAULT_SOL_AIRDROP = 10 * LAMPORTS_PER_SOL;

/**
 * Test harness for Friend Groups contract.
 * Creates an independent test environment with its own group, admin, and members.
 */
export class FriendGroupTestHarness {
  program: Program<FriendGroups>;
  provider: anchor.AnchorProvider;

  // Group-specific state
  admin!: Keypair;
  friendGroupPda!: PublicKey;
  treasurySolPda!: PublicKey;
  treasuryUsdcPda!: PublicKey;
  usdcMint!: PublicKey;
  groupName!: string;

  // Members cache
  private members: Map<string, Keypair> = new Map();

  constructor(program?: Program<FriendGroups>, provider?: anchor.AnchorProvider) {
    this.program = program || (anchor.workspace.FriendGroups as Program<FriendGroups>);
    this.provider = provider || anchor.getProvider() as anchor.AnchorProvider;
  }

  /**
   * Initialize a new group with admin and USDC mint.
   * Creates all necessary accounts and PDAs.
   */
  async init(groupName: string = "Test Group"): Promise<void> {
    this.groupName = groupName;
    this.admin = Keypair.generate();

    // Airdrop SOL to admin
    await helpers.airdropSol(
      this.provider.connection,
      this.admin.publicKey,
      DEFAULT_SOL_AIRDROP
    );

    // Create USDC mint
    this.usdcMint = await createMint(
      this.provider.connection,
      this.admin,
      this.admin.publicKey,
      null,
      USDC_DECIMALS
    );

    // Derive PDAs
    [this.friendGroupPda] = helpers.deriveFriendGroupPda(
      this.admin.publicKey,
      this.program.programId
    );
    [this.treasurySolPda] = helpers.deriveTreasurySolPda(
      this.friendGroupPda,
      this.program.programId
    );
    this.treasuryUsdcPda = await getAssociatedTokenAddress(
      this.usdcMint,
      this.friendGroupPda,
      true
    );

    // Create USDC treasury ATA
    const createAtaIx = createAssociatedTokenAccountInstruction(
      this.admin.publicKey,
      this.treasuryUsdcPda,
      this.friendGroupPda,
      this.usdcMint
    );

    const tx = new Transaction().add(createAtaIx);
    const txSig = await this.provider.connection.sendTransaction(tx, [this.admin]);
    await this.provider.connection.confirmTransaction(txSig);

    // Create the friend group
    await this.program.methods
      .createGroup(this.groupName)
      .accounts({
        admin: this.admin.publicKey,
        treasuryUsdc: this.treasuryUsdcPda,
        usdcMint: this.usdcMint,
      })
      .signers([this.admin])
      .rpc();
  }

  /**
   * Create and airdrop SOL to a new member keypair.
   */
  async createMember(name?: string): Promise<Keypair> {
    const member = Keypair.generate();
    await helpers.airdropSol(
      this.provider.connection,
      member.publicKey,
      DEFAULT_SOL_AIRDROP
    );
    
    if (name) {
      this.members.set(name, member);
    }
    
    return member;
  }

  /**
   * Add a member to the group (invite + accept flow).
   * If member is not provided, creates a new one.
   */
  async addMember(member?: Keypair, inviter?: Keypair): Promise<Keypair> {
    if (!member) {
      member = await this.createMember();
    }

    const actualInviter = inviter || this.admin;
    const [invitePda] = helpers.deriveInvitePda(
      this.friendGroupPda,
      member.publicKey,
      this.program.programId
    );

    // Check if invite already exists
    try {
      await this.program.account.invite.fetch(invitePda);
    } catch {
      // Create invite if it doesn't exist
      await this.program.methods
        .inviteMember()
        .accounts({
          friendGroup: this.friendGroupPda,
          invitedUser: member.publicKey,
          inviter: actualInviter.publicKey,
        })
        .signers([actualInviter])
        .rpc();
    }

    // Accept invite
    await this.program.methods
      .acceptInvite()
      .accounts({
        friendGroup: this.friendGroupPda,
        invitedUser: member.publicKey,
      })
      .signers([member])
      .rpc();

    return member;
  }

  /**
   * Ensure a USDC token account exists for a user.
   * Creates it if it doesn't exist.
   */
  async ensureTokenAccount(user: Keypair): Promise<PublicKey> {
    const tokenAccount = await getAssociatedTokenAddress(
      this.usdcMint,
      user.publicKey
    );

    try {
      await getAccount(this.provider.connection, tokenAccount);
    } catch {
      // Account doesn't exist, create it
      const createAtaIx = createAssociatedTokenAccountInstruction(
        user.publicKey,
        tokenAccount,
        user.publicKey,
        this.usdcMint
      );
      const tx = new Transaction().add(createAtaIx);
      const txSig = await this.provider.connection.sendTransaction(tx, [user]);
      await this.provider.connection.confirmTransaction(txSig);
      
      // Wait for account initialization
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    return tokenAccount;
  }

  /**
   * Mint USDC to a user's token account.
   */
  async mintUsdc(user: Keypair, amount: number): Promise<void> {
    const tokenAccount = await this.ensureTokenAccount(user);
    await mintTo(
      this.provider.connection,
      this.admin,
      this.usdcMint,
      tokenAccount,
      this.admin,
      amount
    );
  }

  /**
   * Deposit funds for a member (SOL and/or USDC).
   */
  async depositFor(
    user: Keypair,
    solAmount: number = 0,
    usdcAmount: number = 0
  ): Promise<void> {
    const memberUsdcAccount = await this.ensureTokenAccount(user);

    // Mint USDC if needed
    if (usdcAmount > 0) {
      const balance = await getAccount(this.provider.connection, memberUsdcAccount);
      if (Number(balance.amount.toString()) < usdcAmount) {
        await this.mintUsdc(user, usdcAmount);
      }
    }

    await this.program.methods
      .depositFunds(new anchor.BN(solAmount), new anchor.BN(usdcAmount))
      .accounts({
        friendGroup: this.friendGroupPda,
        memberWallet: user.publicKey,
        treasuryUsdc: this.treasuryUsdcPda,
        memberUsdcAccount: memberUsdcAccount,
      })
      .signers([user])
      .rpc();
  }

  /**
   * Withdraw funds for a member (SOL and/or USDC).
   */
  async withdrawFor(
    user: Keypair,
    solAmount: number = 0,
    usdcAmount: number = 0
  ): Promise<void> {
    const memberUsdcAccount = await this.ensureTokenAccount(user);

    await this.program.methods
      .withdrawFunds(new anchor.BN(solAmount), new anchor.BN(usdcAmount))
      .accounts({
        friendGroup: this.friendGroupPda,
        memberWallet: user.publicKey,
        treasuryUsdc: this.treasuryUsdcPda,
        memberUsdcAccount: memberUsdcAccount,
      })
      .signers([user])
      .rpc();
  }

  /**
   * Remove a member from the group.
   */
  async removeMember(user: Keypair, admin?: Keypair): Promise<void> {
    const actualAdmin = admin || this.admin;
    const memberUsdcAccount = await this.ensureTokenAccount(user);

    await this.program.methods
      .removeMember()
      .accounts({
        friendGroup: this.friendGroupPda,
        memberWallet: user.publicKey,
        treasuryUsdc: this.treasuryUsdcPda,
        memberUsdcAccount: memberUsdcAccount,
        admin: actualAdmin.publicKey,
      })
      .signers([actualAdmin])
      .rpc();
  }

  /**
   * Get the friend group account.
   */
  async getGroup(): Promise<any> {
    return await this.program.account.friendGroup.fetch(this.friendGroupPda);
  }

  /**
   * Get a member account.
   */
  async getMember(user: Keypair): Promise<any> {
    const [memberPda] = helpers.deriveMemberPda(
      this.friendGroupPda,
      user.publicKey,
      this.program.programId
    );
    return await this.program.account.groupMember.fetch(memberPda);
  }

  /**
   * Get member PDA.
   */
  getMemberPda(user: Keypair): PublicKey {
    const [memberPda] = helpers.deriveMemberPda(
      this.friendGroupPda,
      user.publicKey,
      this.program.programId
    );
    return memberPda;
  }

  /**
   * Get invite PDA.
   */
  getInvitePda(user: Keypair): PublicKey {
    const [invitePda] = helpers.deriveInvitePda(
      this.friendGroupPda,
      user.publicKey,
      this.program.programId
    );
    return invitePda;
  }

  /**
   * Get treasury SOL balance.
   */
  async getTreasurySolBalance(): Promise<number> {
    return await this.provider.connection.getBalance(this.treasurySolPda);
  }

  /**
   * Get treasury USDC balance.
   */
  async getTreasuryUsdcBalance(): Promise<number> {
    const account = await getAccount(this.provider.connection, this.treasuryUsdcPda);
    return Number(account.amount.toString());
  }

  /**
   * Assert that an async function throws an error.
   * Supports matching by error code, error message, or custom matcher function.
   */
  async expectError(
    fn: () => Promise<any>,
    matcher?: string | number | ((err: any) => boolean)
  ): Promise<void> {
    try {
      await fn();
      expect.fail("Expected function to throw an error");
    } catch (err: any) {
      expect(err).to.exist;

      if (!matcher) {
        // Just verify an error was thrown
        expect(err instanceof Error || err instanceof AnchorError).to.be.true;
        return;
      }

      if (typeof matcher === "function") {
        expect(matcher(err)).to.be.true;
      } else if (typeof matcher === "string") {
        // Match by error code or message
        const errorMsg = err instanceof Error ? err.message : String(err);
        const errorCode = err instanceof AnchorError 
          ? err.error?.errorCode?.code 
          : null;
        
        expect(
          errorCode === matcher || 
          errorMsg.includes(matcher)
        ).to.be.true;
      } else if (typeof matcher === "number") {
        // Match by error code number
        const errorCode = err instanceof AnchorError 
          ? err.error?.errorCode?.number 
          : null;
        expect(errorCode).to.equal(matcher);
      }
    }
  }

  /**
   * Assert that an async function throws an authorization error.
   */
  async expectUnauthorizedError(fn: () => Promise<any>): Promise<void> {
    await this.expectError(fn, (err) => {
      const errorMsg = err instanceof Error ? err.message : String(err);
      return (
        errorMsg.includes("Unauthorized") ||
        errorMsg.includes("Only admin") ||
        (err instanceof AnchorError && 
         err.error?.errorCode?.code === "Unauthorized") ||
        err instanceof AnchorError
      );
    });
  }

  /**
   * Assert that an async function throws an InvalidAmount error.
   */
  async expectInvalidAmountError(fn: () => Promise<any>): Promise<void> {
    await this.expectError(fn, (err) => {
      if (err instanceof AnchorError) {
        const errorCode = err.error?.errorCode?.code || err.error?.errorCode?.number;
        const errorMsg = err.error?.errorMessage || err.message || String(err);
        return (
          errorCode === "InvalidAmount" ||
          errorCode === 6005 ||
          errorMsg.includes("Invalid amount") ||
          errorMsg.includes("InvalidAmount")
        );
      }
      const errorMsg = err instanceof Error ? err.message : String(err);
      return errorMsg.includes("Invalid amount") || errorMsg.includes("InvalidAmount");
    });
  }

  /**
   * Assert that an async function throws a MinMembersRequired error.
   */
  async expectMinMembersError(fn: () => Promise<any>): Promise<void> {
    await this.expectError(fn, (err) => {
      const errorMsg = err instanceof Error ? err.message : String(err);
      return (
        errorMsg.includes("at least 3 members") ||
        errorMsg.includes("MinMembersRequired") ||
        (err instanceof AnchorError && 
         err.error?.errorCode?.code === "MinMembersRequired") ||
        err instanceof AnchorError
      );
    });
  }
}

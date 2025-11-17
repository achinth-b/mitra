import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { Events } from "../target/types/events";
import { FriendGroups } from "../target/types/friend_groups";
import {
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  createMint,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { expect } from "chai";
import * as helpers from "./helpers";

// Test constants
const TEST_CONSTANTS = {
  SOL_AIRDROP_AMOUNT: 10 * LAMPORTS_PER_SOL,
  USDC_DECIMALS: 6,
  DEFAULT_DAYS_UNTIL_RESOLVE: 7,
  LONG_TITLE_LENGTH: 101,
} as const;

const EVENT_TITLE = "Will it rain tomorrow?";
const EVENT_DESCRIPTION = "Simple weather prediction";
const EVENT_OUTCOMES = ["YES", "NO"];

describe("Events", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const eventsProgram = anchor.workspace.Events as Program<Events>;
  const friendGroupsProgram = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider();

  let admin: Keypair;
  let member1: Keypair;
  let nonMember: Keypair;
  let usdcMint: PublicKey;
  let friendGroupPda: PublicKey;
  let treasuryUsdcPda: PublicKey;
  let backendAuthority: Keypair;

  // Setup helper functions
  async function setupTestAccounts() {
    admin = Keypair.generate();
    member1 = Keypair.generate();
    nonMember = Keypair.generate();
    backendAuthority = Keypair.generate();

    const accounts = [admin, member1, nonMember, backendAuthority];
    await Promise.all(
      accounts.map(account => 
        helpers.airdropSol(
          provider.connection,
          account.publicKey,
          TEST_CONSTANTS.SOL_AIRDROP_AMOUNT
        )
      )
    );
  }

  async function setupUsdcToken() {
    usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      TEST_CONSTANTS.USDC_DECIMALS
    );
  }

  async function setupFriendGroup() {
    [friendGroupPda] = helpers.deriveFriendGroupPda(
      admin.publicKey,
      friendGroupsProgram.programId
    );
    treasuryUsdcPda = await getAssociatedTokenAddress(
      usdcMint,
      friendGroupPda,
      true
    );

    const createAtaIx = createAssociatedTokenAccountInstruction(
      admin.publicKey,
      treasuryUsdcPda,
      friendGroupPda,
      usdcMint
    );

    const tx = new Transaction().add(createAtaIx);
    const txSig = await provider.connection.sendTransaction(tx, [admin]);
    await provider.connection.confirmTransaction(txSig);

    await (friendGroupsProgram.methods as any)
      .createGroup("Test Group")
      .accounts({
        admin: admin.publicKey,
        treasuryUsdc: treasuryUsdcPda,
        usdcMint: usdcMint,
      })
      .signers([admin])
      .rpc();
  }

  before(async () => {
    await setupTestAccounts();
    await setupUsdcToken();
    await setupFriendGroup();
  });

  describe("create_event", () => {
    it("Successfully creates an event", async () => {
      const { eventPda } = await helpers.createTestEvent(
        eventsProgram,
        friendGroupPda,
        EVENT_TITLE,
        admin,
        EVENT_DESCRIPTION,
        EVENT_OUTCOMES
      );

      const eventAccount = await eventsProgram.account.eventContract.fetch(eventPda);
      expect(eventAccount.title).to.equal(EVENT_TITLE);
      expect(eventAccount.description).to.equal(EVENT_DESCRIPTION);
      expect(eventAccount.outcomes).to.deep.equal(EVENT_OUTCOMES);
      expect(eventAccount.status).to.deep.equal({ active: {} });
      expect(eventAccount.settlementType).to.deep.equal({ manual: {} });
    });

    it("Fails when non-admin tries to create event", async () => {
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + TEST_CONSTANTS.DEFAULT_DAYS_UNTIL_RESOLVE);

      const [eventPda] = helpers.deriveEventPda(
        friendGroupPda,
        "Unauthorized Event",
        eventsProgram.programId
      );
      const [eventStatePda] = helpers.deriveEventStatePda(
        eventPda,
        eventsProgram.programId
      );

      try {
        await eventsProgram.methods
          .createEvent(
            "Unauthorized Event",
            "Should fail",
            EVENT_OUTCOMES,
            { manual: {} },
            new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
          )
          .accounts({
            eventContract: eventPda,
            eventState: eventStatePda,
            group: friendGroupPda,
            admin: nonMember.publicKey,
          } as any)
          .signers([nonMember])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "Unauthorized");
      }
    });

    it("Fails with title too long", async () => {
      const longTitle = "a".repeat(TEST_CONSTANTS.LONG_TITLE_LENGTH);
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + TEST_CONSTANTS.DEFAULT_DAYS_UNTIL_RESOLVE);

      const [eventPda] = helpers.deriveEventPda(
        friendGroupPda,
        longTitle,
        eventsProgram.programId
      );
      const [eventStatePda] = helpers.deriveEventStatePda(
        eventPda,
        eventsProgram.programId
      );

      try {
        await eventsProgram.methods
          .createEvent(
            longTitle,
            EVENT_DESCRIPTION,
            EVENT_OUTCOMES,
            { manual: {} },
            new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
          )
          .accounts({
            eventContract: eventPda,
            eventState: eventStatePda,
            group: friendGroupPda,
            admin: admin.publicKey,
          } as any)
          .signers([admin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err: any) {
        let errorCode: string;
        let errorNumber: number;
        
        if (err instanceof AnchorError) {
          errorCode = err.error.errorCode.code;
          errorNumber = err.error.errorCode.number;
        } else {
          errorCode = err.error?.errorCode?.code || err.errorCode?.code;
          errorNumber = err.error?.errorCode?.number || err.errorCode?.number;
        }
        
        expect(errorCode).to.equal("TitleTooLong");
        expect(errorNumber).to.equal(6006);
      }
    });
  });

  describe("commit_state", () => {
    let eventPda: PublicKey;
    let eventStatePda: PublicKey;

    before(async () => {
      const result = await helpers.createTestEvent(
        eventsProgram,
        friendGroupPda,
        "Commit Test Event",
        admin,
        "Testing commit state",
        EVENT_OUTCOMES
      );
      eventPda = result.eventPda;
      eventStatePda = result.eventStatePda;
    });

    it("Successfully commits merkle root", async () => {
      const merkleRoot = Buffer.alloc(32, 1); // Dummy merkle root

      await eventsProgram.methods
        .commitState(Array.from(merkleRoot))
        .accounts({
          eventContract: eventPda,
          eventState: eventStatePda,
          backendAuthority: backendAuthority.publicKey,
        } as any)
        .signers([backendAuthority])
        .rpc();

      const eventState = await eventsProgram.account.eventState.fetch(eventStatePda);
      expect(eventState.lastMerkleRoot).to.deep.equal(Array.from(merkleRoot));
      expect(eventState.lastCommitSlot.toNumber()).to.be.greaterThan(0);
    });
  });

  describe("settle_event", () => {
    let eventPda: PublicKey;

    before(async () => {
      const result = await helpers.createTestEvent(
        eventsProgram,
        friendGroupPda,
        "Settle Test Event",
        admin,
        "Testing settle event",
        EVENT_OUTCOMES
      );
      eventPda = result.eventPda;
    });

    it("Successfully settles event", async () => {
      await eventsProgram.methods
        .settleEvent("YES")
        .accounts({
          eventContract: eventPda,
          group: friendGroupPda,
          admin: admin.publicKey,
        } as any)
        .signers([admin])
        .rpc();

      const event = await eventsProgram.account.eventContract.fetch(eventPda);
      expect(event.status).to.deep.equal({ resolved: {} });
      expect(event.winningOutcome).to.equal("YES");
      expect(event.settledAt).to.not.be.null;
    });

    it("Fails when non-admin tries to settle", async () => {
      const { eventPda: newEventPda } = await helpers.createTestEvent(
        eventsProgram,
        friendGroupPda,
        "Unauthorized Settle",
        admin,
        "Testing unauthorized settle",
        EVENT_OUTCOMES
      );

      try {
        await eventsProgram.methods
          .settleEvent("YES")
          .accounts({
            eventContract: newEventPda,
            group: friendGroupPda,
            admin: nonMember.publicKey,
          } as any)
          .signers([nonMember])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "Unauthorized");
      }
    });

    it("Fails when event already settled", async () => {
      try {
        await eventsProgram.methods
          .settleEvent("NO")
          .accounts({
            eventContract: eventPda,
            group: friendGroupPda,
            admin: admin.publicKey,
          })
          .signers([admin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err: any) {
        helpers.assertAnchorError(err, "EventAlreadySettled");
      }
    });
  });

  describe("claim_winnings", () => {
    // This test requires actual bet/winnings logic which will be in backend
    // For now, we'll test the basic structure
    it("Placeholder for claim_winnings tests", async () => {
      // TODO: Implement when bet settlement logic is ready
      expect(true).to.be.true;
    });
  });
});
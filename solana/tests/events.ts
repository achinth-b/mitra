import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Events } from "../target/types/events";
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

describe("Events", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const eventsProgram = anchor.workspace.Events as Program<Events>;
  const friendGroupsProgram = anchor.workspace.FriendGroups as Program<FriendGroups>;
  const provider = anchor.getProvider();

  let admin: Keypair;
  let member1: Keypair;
  let nonMember: Keypair;
  let usdcMint: PublicKey;
  let usdcToken: Token;
  let friendGroupPda: PublicKey;
  let treasuryUsdcPda: PublicKey;
  let backendAuthority: Keypair;

  const EVENT_TITLE = "Will it rain tomorrow?";
  const EVENT_DESCRIPTION = "Simple weather prediction";
  const EVENT_OUTCOMES = ["YES", "NO"];

  before(async () => {
    admin = Keypair.generate();
    member1 = Keypair.generate();
    nonMember = Keypair.generate();
    backendAuthority = Keypair.generate();

    await helpers.airdropSol(provider.connection, admin.publicKey, 10 * LAMPORTS_PER_SOL);
    await helpers.airdropSol(provider.connection, member1.publicKey, 10 * LAMPORTS_PER_SOL);
    await helpers.airdropSol(provider.connection, nonMember.publicKey, 10 * LAMPORTS_PER_SOL);
    await helpers.airdropSol(provider.connection, backendAuthority.publicKey, 10 * LAMPORTS_PER_SOL);

    usdcToken = await Token.createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6,
      TOKEN_PROGRAM_ID
    );
    usdcMint = usdcToken.publicKey;

    [friendGroupPda] = helpers.deriveFriendGroupPda(admin.publicKey, friendGroupsProgram.programId);
    treasuryUsdcPda = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      usdcMint,
      friendGroupPda,
      true
    );

    // Create friend group first
    const createAtaIx = Token.createAssociatedTokenAccountInstruction(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      usdcMint,
      treasuryUsdcPda,
      friendGroupPda,
      admin.publicKey
    );

    const tx = new Transaction().add(createAtaIx);
    await provider.connection.sendTransaction(tx, [admin]);

    await (friendGroupsProgram.methods as any)
      .createGroup("Test Group")
      .accounts({
        admin: admin.publicKey,
        treasuryUsdc: treasuryUsdcPda,
        usdcMint: usdcMint,
      })
      .signers([admin])
      .rpc();
  });

  describe("create_event", () => {
    it("Successfully creates an event", async () => {
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7); // 7 days from now

      const [eventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from(EVENT_TITLE),
        ],
        eventsProgram.programId
      );

      const [eventStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("event_state"), eventPda.toBuffer()],
        eventsProgram.programId
      );

      await eventsProgram.methods
        .createEvent(
          EVENT_TITLE,
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

      const eventAccount = await eventsProgram.account.eventContract.fetch(eventPda);
      expect(eventAccount.title).to.equal(EVENT_TITLE);
      expect(eventAccount.description).to.equal(EVENT_DESCRIPTION);
      expect(eventAccount.outcomes).to.deep.equal(EVENT_OUTCOMES);
      expect(eventAccount.status).to.deep.equal({ active: {} });
      expect(eventAccount.settlementType).to.deep.equal({ manual: {} });
    });

    it("Fails when non-admin tries to create event", async () => {
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7);

      const [eventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from("Unauthorized Event"),
        ],
        eventsProgram.programId
      );

      try {
        await eventsProgram.methods
          .createEvent(
            "Unauthorized Event",
            "Should fail",
            ["YES", "NO"],
            { manual: {} },
            new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
          )
          .accounts({
            eventContract: eventPda,
            eventState: PublicKey.findProgramAddressSync(
              [Buffer.from("event_state"), eventPda.toBuffer()],
              eventsProgram.programId
            )[0],
            group: friendGroupPda,
            admin: nonMember.publicKey,
          } as any)
          .signers([nonMember])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("Unauthorized");
      }
    });

    it("Fails with title too long", async () => {
      const longTitle = "a".repeat(101);
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7);

      const [eventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from(longTitle),
        ],
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
            eventState: PublicKey.findProgramAddressSync(
              [Buffer.from("event_state"), eventPda.toBuffer()],
              eventsProgram.programId
            )[0],
            group: friendGroupPda,
            admin: admin.publicKey,
          } as any)
          .signers([admin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("Title too long");
      }
    });
  });

  describe("commit_state", () => {
    let eventPda: PublicKey;
    let eventStatePda: PublicKey;

    before(async () => {
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7);

      [eventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from("Commit Test Event"),
        ],
        eventsProgram.programId
      );

      [eventStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("event_state"), eventPda.toBuffer()],
        eventsProgram.programId
      );

      await eventsProgram.methods
        .createEvent(
          "Commit Test Event",
          "Testing commit state",
          EVENT_OUTCOMES,
          { manual: {} },
          new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
        )
        .accounts({
          eventContract: eventPda,
          eventState: PublicKey.findProgramAddressSync(
            [Buffer.from("event_state"), eventPda.toBuffer()],
            eventsProgram.programId
          )[0],
          group: friendGroupPda,
          admin: admin.publicKey,
        } as any)
        .signers([admin])
        .rpc();
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
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7);

      [eventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from("Settle Test Event"),
        ],
        eventsProgram.programId
      );

      await eventsProgram.methods
        .createEvent(
          "Settle Test Event",
          "Testing settle event",
          EVENT_OUTCOMES,
          { manual: {} },
          new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
        )
        .accounts({
          eventContract: eventPda,
          eventState: PublicKey.findProgramAddressSync(
            [Buffer.from("event_state"), eventPda.toBuffer()],
            eventsProgram.programId
          )[0],
          group: friendGroupPda,
          admin: admin.publicKey,
        } as any)
        .signers([admin])
        .rpc();
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
      // Create new event for this test
      const resolveBy = new Date();
      resolveBy.setDate(resolveBy.getDate() + 7);

      const [newEventPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("event"),
          friendGroupPda.toBuffer(),
          Buffer.from("Unauthorized Settle"),
        ],
        eventsProgram.programId
      );

      await eventsProgram.methods
        .createEvent(
          "Unauthorized Settle",
          "Testing unauthorized settle",
          EVENT_OUTCOMES,
          { manual: {} },
          new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
        )
        .accounts({
          eventContract: newEventPda,
          eventState: PublicKey.findProgramAddressSync(
            [Buffer.from("event_state"), newEventPda.toBuffer()],
            eventsProgram.programId
          )[0],
          group: friendGroupPda,
          admin: admin.publicKey,
        } as any)
        .signers([admin])
        .rpc();

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
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("Unauthorized");
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
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        expect(errorMsg).to.include("already settled");
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
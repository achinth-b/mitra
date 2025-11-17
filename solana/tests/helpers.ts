import * as anchor from "@coral-xyz/anchor";
import { Events } from "../target/types/events";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import pkg from "js-sha3";
const { keccak256 } = pkg;

export async function airdropSol(
  connection: anchor.web3.Connection,
  pubkey: PublicKey,
  amount: number
): Promise<void> {
  const signature = await connection.requestAirdrop(pubkey, amount);
  await connection.confirmTransaction(signature);
}

export function deriveFriendGroupPda(
  admin: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("friend_group"), admin.toBuffer()],
    programId
  );
}

export function deriveTreasurySolPda(
  friendGroup: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("treasury_sol"), friendGroup.toBuffer()],
    programId
  );
}

export function deriveMemberPda(
  friendGroup: PublicKey,
  user: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("member"), friendGroup.toBuffer(), user.toBuffer()],
    programId
  );
}

export function deriveInvitePda(
  friendGroup: PublicKey,
  invitedUser: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("invite"), friendGroup.toBuffer(), invitedUser.toBuffer()],
    programId
  );
}

export function deriveEventPda(
    group: PublicKey,
    title: string,
    programId: PublicKey
  ): [PublicKey, number] {
    // Hash title using keccak256 (matches Rust constraint)
    const titleHash = Buffer.from(keccak256(title), "hex");
    return PublicKey.findProgramAddressSync(
      [Buffer.from("event"), group.toBuffer(), titleHash],
      programId
    );
  }
  
  export function deriveEventStatePda(
    event: PublicKey,
    programId: PublicKey
  ): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("event_state"), event.toBuffer()],
      programId
    );
  }

/**
 * Assert that an error matches the expected Anchor error code
 */
export function assertAnchorError(err: any, expectedCode: string): void {
  if (err instanceof AnchorError) {
    expect(err.error.errorCode.code).to.equal(expectedCode);
  } else {
    const errorCode = err.error?.errorCode?.code || err.errorCode?.code;
    const errorMsg = err instanceof Error ? err.message : String(err);
    expect(
      errorCode === expectedCode || errorMsg.includes(expectedCode)
    ).to.be.true;
  }
}

/**
 * Helper to create a test event with default parameters
 */
export async function createTestEvent(
  program: Program<Events>,
  friendGroup: PublicKey,
  title: string,
  admin: Keypair,
  description: string = "Test description",
  outcomes: string[] = ["YES", "NO"],
  settlementType: any = { manual: {} },
  daysUntilResolve: number = 7
): Promise<{ eventPda: PublicKey; eventStatePda: PublicKey }> {
  const resolveBy = new Date();
  resolveBy.setDate(resolveBy.getDate() + daysUntilResolve);

  const [eventPda] = deriveEventPda(friendGroup, title, program.programId);
  const [eventStatePda] = deriveEventStatePda(eventPda, program.programId);

  await program.methods
    .createEvent(
      title,
      description,
      outcomes,
      settlementType,
      new anchor.BN(Math.floor(resolveBy.getTime() / 1000))
    )
    .accounts({
      eventContract: eventPda,
      eventState: eventStatePda,
      group: friendGroup,
      admin: admin.publicKey,
    } as any)
    .signers([admin])
    .rpc();

  return { eventPda, eventStatePda };
}

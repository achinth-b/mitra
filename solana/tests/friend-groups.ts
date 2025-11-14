import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

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
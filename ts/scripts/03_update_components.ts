import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { Program, AnchorProvider, Idl, BN } from "@coral-xyz/anchor";
import { getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { getOrCreateAssociatedTokenAccount as getOrCreateAta2022, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token-2022";
import dotenv from "dotenv";
import bs58 from "bs58";
import idl from "../idl/basket.json" assert { type: "json" };

dotenv.config();

function toQ64(num: number): bigint {
  return BigInt(Math.round(num * Math.pow(2, 32))) << BigInt(32);
}

// Sets the basket NAV via the oracle adapter (admin only). You must set NAV_USD in .env.
(async () => {
  const conn = new Connection(process.env.RPC_URL || "https://api.devnet.solana.com", "confirmed");
  const secret = process.env.PAYER_SECRET;
  if (!secret) {
    console.error("Set PAYER_SECRET in your .env file");
    return;
  }
  const payer = Keypair.fromSecretKey(bs58.decode(secret));
  const provider = new AnchorProvider(conn, { publicKey: payer.publicKey } as any, {} as any);
  const program = new Program(idl as Idl, new PublicKey((idl as any).metadata.address), provider);

  const baseMintStr = process.env.BASE_MINT;
  const basketMintStr = process.env.BASKET_MINT;
  if (!baseMintStr || !basketMintStr) {
    console.error("Set BASE_MINT and BASKET_MINT in your .env file");
    return;
  }
  const baseMint = new PublicKey(baseMintStr);
  const basketMint = new PublicKey(basketMintStr);
  const [basketPda] = PublicKey.findProgramAddressSync([
    Buffer.from("basket"),
    basketMint.toBuffer(),
  ], program.programId);
  const baseVault = (await getOrCreateAssociatedTokenAccount(conn, payer, baseMint, basketPda, true)).address;
  const adminBaseAta = (await getOrCreateAssociatedTokenAccount(conn, payer, baseMint, payer.publicKey)).address;
  // NAV to Q64.64
  const nav = Number(process.env.NAV_USD || "1.00");
  const navQ64 = toQ64(nav);
  await program.methods.adminSetNavQ64(new BN(navQ64.toString()))
    .accounts({
      admin: payer.publicKey,
      basket: basketPda,
      baseMint,
      basketMint,
      baseVault,
      adminBaseAta,
      tokenProgram: TOKEN_PROGRAM_ID,
      token2022Program: TOKEN_2022_PROGRAM_ID,
    })
    .signers([payer])
    .rpc();
  console.log(`Set NAV to ${nav} (Q64=${navQ64.toString()})`);
})();
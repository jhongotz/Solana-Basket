import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { getOrCreateAssociatedTokenAccount as getOrCreateAta2022, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token-2022";
import { Program, AnchorProvider, Idl, BN } from "@coral-xyz/anchor";
import dotenv from "dotenv";
import bs58 from "bs58";
import idl from "../idl/basket.json" assert { type: "json" };

dotenv.config();

// Mint basket shares by depositing base tokens.
(async () => {
  const conn = new Connection(process.env.RPC_URL || "https://api.devnet.solana.com", "confirmed");
  const secret = process.env.PAYER_SECRET;
  if (!secret) {
    console.error("Set PAYER_SECRET in .env");
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
  // User base ATA and basket ATA
  const userBaseAta = (await getOrCreateAssociatedTokenAccount(conn, payer, baseMint, payer.publicKey)).address;
  const userBasketAta = (await getOrCreateAta2022(conn, payer, basketMint, payer.publicKey, undefined, TOKEN_2022_PROGRAM_ID)).address;
  // Derive user position PDA
  const [userPosPda] = PublicKey.findProgramAddressSync([
    Buffer.from("pos"),
    basketPda.toBuffer(),
    payer.publicKey.toBuffer(),
  ], program.programId);
  const baseIn = new BN(100_000); // deposit 0.1 units (assuming 6 decimals)
  const minSharesOut = new BN(1);
  await program.methods.mintShares(baseIn, minSharesOut)
    .accounts({
      payer: payer.publicKey,
      basket: basketPda,
      baseVault,
      userBaseAta,
      userBasketAta,
      baseMint,
      basketMint,
      userPosition: userPosPda,
      tokenProgram: TOKEN_PROGRAM_ID,
      token2022Program: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([payer])
    .rpc();
  console.log("Minted shares");
})();
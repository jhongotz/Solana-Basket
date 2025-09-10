import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import dotenv from "dotenv";
import bs58 from "bs58";

dotenv.config();

// Initializes a new base mint (e.g. a dev stablecoin) and mints tokens to the admin.
(async () => {
  const conn = new Connection(process.env.RPC_URL || "https://api.devnet.solana.com", "confirmed");
  const secret = process.env.PAYER_SECRET;
  if (!secret) {
    console.error("Set PAYER_SECRET in your .env file");
    return;
  }
  const payer = Keypair.fromSecretKey(bs58.decode(secret));

  // Number of decimals on the base mint (default 6).
  const decimals = Number(process.env.BASE_DECIMALS || 6);
  // Create mint; admin is both mint and freeze authority.
  const baseMint = await createMint(conn, payer, payer.publicKey, null, decimals);
  // Create ATA for admin and mint 1000 base units.
  const baseAta = await getOrCreateAssociatedTokenAccount(conn, payer, baseMint, payer.publicKey);
  const amount = BigInt(1_000_000 * 10 ** (decimals - 6)); // 1000 units scaled
  await mintTo(conn, payer, baseMint, baseAta.address, payer, Number(amount));
  console.log("BASE_MINT=" + baseMint.toBase58());
  console.log("BASE_ATA=" + baseAta.address.toBase58());
})();
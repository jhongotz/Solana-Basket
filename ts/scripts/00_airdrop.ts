import { Connection, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import dotenv from "dotenv";
import bs58 from "bs58";

dotenv.config();

// Simple script to airdrop SOL to the admin/payer for devnet testing.
(async () => {
  const url = process.env.RPC_URL || "https://api.devnet.solana.com";
  const conn = new Connection(url, "confirmed");
  const secret = process.env.PAYER_SECRET;
  if (!secret) {
    console.error("Please set PAYER_SECRET in your .env file");
    return;
  }
  const payer = Keypair.fromSecretKey(bs58.decode(secret));
  const sig = await conn.requestAirdrop(payer.publicKey, 2n * BigInt(LAMPORTS_PER_SOL));
  await conn.confirmTransaction(sig, "confirmed");
  console.log(`Airdropped 2 SOL to ${payer.publicKey.toBase58()}`);
})();
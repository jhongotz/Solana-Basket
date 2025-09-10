import { Keypair, Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount } from "@solana/spl-token-2022";
import { Program, AnchorProvider, Idl } from "@coral-xyz/anchor";
import dotenv from "dotenv";
import bs58 from "bs58";

// NOTE: You must run `anchor build` and copy the generated IDL into ts/idl/basket.json
// before using this script. The IDL provides instruction layouts and program ID.
import idl from "../idl/basket.json" assert { type: "json" };

dotenv.config();

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

  // Retrieve base mint from env
  const baseMintStr = process.env.BASE_MINT;
  if (!baseMintStr) {
    console.error("Set BASE_MINT in your .env file (run init:base first)");
    return;
  }
  const baseMint = new PublicKey(baseMintStr);

  // Create basket mint (token2022) with same decimals as base mint (defaults to 6)
  const decimals = Number(process.env.BASE_DECIMALS || 6);
  const basketMint = await createMint(conn, payer, undefined as any, null, decimals, undefined, undefined, TOKEN_2022_PROGRAM_ID);

  // Derive the basket PDA
  const [basketPda] = PublicKey.findProgramAddressSync([
    Buffer.from("basket"),
    basketMint.toBuffer(),
  ], program.programId);

  // Create base vault ATA for basket PDA
  const baseVault = (await getOrCreateAssociatedTokenAccount(
    conn,
    payer,
    baseMint,
    basketPda,
    true
  )).address;

  // Call create_basket instruction on the program
  await program.methods.createBasket(50 /* mgmt fee in bps */)
    .accounts({
      admin: payer.publicKey,
      basket: basketPda,
      baseMint,
      basketMint,
      baseVault,
      systemProgram: SystemProgram.programId,
    })
    .signers([payer])
    .rpc();

  console.log("BASKET_MINT=" + basketMint.toBase58());
  console.log("BASKET_PDA=" + basketPda.toBase58());
  console.log("BASE_VAULT=" + baseVault.toBase58());
})();
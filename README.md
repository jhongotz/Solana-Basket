# Solana Basket MVP

This repository contains a minimal working proof‑of‑concept for a tokenized stock basket on Solana.  It is designed for Devnet testing and demonstrates how to:

* Create a **basket token** (SPL Token‑2022 mint) with a **vault** that holds a base token (e.g. a dev stablecoin).
* **Mint** and **redeem** basket shares for the base token according to a Net Asset Value (NAV).
* **Accrue dividends** and allow users to **claim** them pro rata.
* Maintain **synthetic NAV** via an admin‑set price (placeholder for an oracle).
* Provide a simple **KYC registry** (compliance) for future transfer hooks.

⚠️ **Important:** This MVP uses a synthetic model of the basket.  In production you would integrate with a price oracle, off‑chain brokers, and implement Token‑2022 transfer hooks for KYC/AML compliance.  Use Devnet only; do not deploy to mainnet without proper audits.

## Repository Layout

```
solana-basket-mvp/
├─ Anchor.toml              # Anchor workspace configuration
├─ Cargo.toml               # Workspace manifest
├─ programs/               # On‑chain programs (Anchor)
│  ├─ basket/              # Basket program (mint/redeem/dividends)
│  ├─ oracle_adapter/      # Minimal oracle to set NAV
│  └─ compliance/          # KYC registry (no transfer hook yet)
├─ ts/                     # TypeScript scripts for testing
│  ├─ .env.example         # Environment variables
│  ├─ package.json         # npm dependencies and scripts
│  ├─ tsconfig.json        # ts-node/TypeScript config
│  └─ scripts/            # Utility scripts:
│     ├─ 00_airdrop.ts         # Airdrop SOL to the admin on Devnet
│     ├─ 01_init_base_mint.ts  # Create a dev stablecoin mint and fund admin
│     ├─ 02_init_basket.ts     # Create the basket PDA, mint and vault
│     ├─ 03_update_components.ts # Set the basket NAV (synthetic)
│     ├─ 04_mint_shares.ts     # Mint basket shares by depositing base
│     ├─ 05_redeem_shares.ts   # Redeem basket shares for base
│     ├─ 06_deposit_dividends.ts # Admin deposits dividends
│     └─ 07_claim_dividends.ts   # User claims outstanding dividends
└─ idl/                   # Placeholder for generated IDLs (see below)
```

## Prerequisites

* **Node.js** (v16+ recommended) and `npm`.
* **ts-node** is installed automatically with `npm install`.
* A **Solana keypair** for Devnet testing.  Use `solana-keygen new --outfile key.json` to generate one.
* **Anchor CLI** and **Rust** are _not_ installed in this container.  To build the on‑chain programs you must install them locally.  Alternatively, you can deploy using `anchor deploy` from your machine and update the program IDs in `Anchor.toml`.

## Setup Instructions

1. **Clone this repository** or copy it to your local machine.

2. **Install npm dependencies** for the TypeScript scripts:

   ```bash
   cd ts
   npm install
   cp .env.example .env    # then edit .env
   ```

   Edit `.env` and set:

   * `PAYER_SECRET` – base58‑encoded secret key for your Devnet keypair.
   * `RPC_URL` (optional) – endpoint for Devnet.
   * After running each init script, update `BASE_MINT` and `BASKET_MINT` accordingly.

3. **Build and deploy the programs**.  From the repository root on your local machine with Anchor installed:

   ```bash
   anchor build
   anchor deploy
   ```

   Take note of the deployed program IDs and update `Anchor.toml` if they differ from the placeholders.

4. **Generate the IDL** for the basket program and copy it into `ts/idl/basket.json`:

   ```bash
   anchor build --idl > ts/idl/basket.json
   ```

   The TypeScript scripts rely on the IDL to know the instruction layouts.

5. **Run the TypeScript scripts** in order to initialize and test the MVP:

   ```bash
   # Airdrop SOL to your admin keypair on Devnet
   npm run airdrop

   # Create base mint and mint some tokens to the admin
   npm run init:base
   # Copy printed BASE_MINT into your .env file

   # Create basket PDA/mint/vault
   npm run init:basket
   # Copy printed BASKET_MINT into your .env file

   # Set synthetic NAV (e.g. 1.00) for the basket
   npm run set:nav

   # Mint basket shares by depositing base tokens
   npm run mint

   # (Optional) Redeem some shares back to base
   npm run redeem

   # Admin deposits dividends into the basket
   npm run div:deposit

   # User claims their accrued dividends
   npm run div:claim
   ```

After these steps, you will have a functioning synthetic basket token on Devnet.  You can inspect balances via `solana token accounts` or explore further enhancements, such as integrating real price feeds, Token‑2022 transfer hooks for KYC, or building a web UI.
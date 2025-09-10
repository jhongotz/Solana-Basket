use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo, Burn};
use anchor_spl::token_2022::Token2022;

declare_id!("BaSket11111111111111111111111111111111111");

/// Helpers for Q64.64 fixed-point math.
fn qmul(a: u128, b: u128) -> u128 {
    // Multiply two 128-bit fixed-point numbers and shift right by 64 to keep scale.
    (a.checked_mul(b).unwrap()) >> 64
}

fn qdiv(a: u128, b: u128) -> u128 {
    // Divide a by b and shift left by 64 to maintain Q64.64 scale.
    (a << 64) / b
}

#[program]
pub mod basket {
    use super::*;

    /// Create a new basket. Admin sets the management fee in basis points.
    pub fn create_basket(ctx: Context<CreateBasket>, mgmt_fee_bps: u16) -> Result<()> {
        let b = &mut ctx.accounts.basket;
        b.admin = ctx.accounts.admin.key();
        b.base_mint = ctx.accounts.base_mint.key();
        b.basket_mint = ctx.accounts.basket_mint.key();
        b.base_vault = ctx.accounts.base_vault.key();
        b.mgmt_fee_bps = mgmt_fee_bps;
        b.acc_div_per_share_q64 = 0;
        b.last_fee_ts = Clock::get()?.unix_timestamp;
        b.paused = false;
        b.nav_per_share_q64 = 0;
        // Store the bump from the PDA to allow signing later.
        b.bump = *ctx.bumps.get("basket").unwrap();
        Ok(())
    }

    /// Admin sets NAV per share (Q64.64). Used in synthetic mode/testing.
    pub fn admin_set_nav_q64(ctx: Context<AdminOnly>, nav_per_share_q64: u128) -> Result<()> {
        require!(!ctx.accounts.basket.paused, BasketError::Paused);
        ctx.accounts.basket.nav_per_share_q64 = nav_per_share_q64;
        Ok(())
    }

    /// User deposits base tokens and mints shares of the basket.
    pub fn mint_shares(ctx: Context<MintShares>, base_in: u64, min_shares_out: u64) -> Result<()> {
        let b = &mut ctx.accounts.basket;
        require!(!b.paused, BasketError::Paused);
        accrue_fees(b)?;
        let nav_q64 = b.nav_per_share_q64;
        require!(nav_q64 > 0, BasketError::StaleOracle);
        let shares_out = ((base_in as u128) << 64) / nav_q64;
        require!(shares_out as u64 >= min_shares_out, BasketError::Slippage);

        // Transfer base from user to basket vault.
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_base_ata.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, base_in)?;

        // Mint basket shares to user.
        let seeds = &[b"basket".as_ref(), b.basket_mint.as_ref(), &[b.bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx2 = CpiContext::new(
            ctx.accounts.token_2022_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.basket_mint.to_account_info(),
                to: ctx.accounts.user_basket_ata.to_account_info(),
                authority: ctx.accounts.basket.to_account_info(),
            },
        ).with_signer(signer_seeds);
        token::mint_to(cpi_ctx2, shares_out as u64)?;

        // Update user's dividend debt.
        let user = &mut ctx.accounts.user_position;
        user.owner = ctx.accounts.payer.key();
        user.basket = b.key();
        let acc = b.acc_div_per_share_q64;
        user.div_debt_q64 = user.div_debt_q64.saturating_add(shares_out.saturating_mul(acc));
        Ok(())
    }

    /// Burn shares and return base tokens to the user based on NAV.
    pub fn redeem_shares(ctx: Context<RedeemShares>, shares_in: u64, min_base_out: u64) -> Result<()> {
        let b = &mut ctx.accounts.basket;
        require!(!b.paused, BasketError::Paused);
        accrue_fees(b)?;
        let nav_q64 = b.nav_per_share_q64;
        require!(nav_q64 > 0, BasketError::StaleOracle);
        let base_out = qmul(shares_in as u128, nav_q64) as u64;
        require!(base_out >= min_base_out, BasketError::Slippage);

        // Burn basket shares.
        let cpi_burn = CpiContext::new(
            ctx.accounts.token_2022_program.to_account_info(),
            Burn {
                mint: ctx.accounts.basket_mint.to_account_info(),
                from: ctx.accounts.user_basket_ata.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::burn(cpi_burn, shares_in)?;

        // Transfer base back to the user.
        let seeds = &[b"basket".as_ref(), b.basket_mint.as_ref(), &[b.bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.base_vault.to_account_info(),
                to: ctx.accounts.user_base_ata.to_account_info(),
                authority: ctx.accounts.basket.to_account_info(),
            },
        ).with_signer(signer_seeds);
        token::transfer(cpi_ctx, base_out)?;
        Ok(())
    }

    /// Deposit dividends into the basket; they are distributed pro rata.
    pub fn deposit_dividends(ctx: Context<AdminOnly>, amount: u64) -> Result<()> {
        let b = &mut ctx.accounts.basket;
        let total_supply = ctx.accounts.basket_mint.supply;
        require!(total_supply > 0, BasketError::NoSupply);
        // Transfer base tokens from admin to vault.
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.admin_base_ata.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, amount)?;
        // Update acc_div_per_share.
        let add = qdiv(amount as u128, total_supply as u128);
        b.acc_div_per_share_q64 = b.acc_div_per_share_q64.saturating_add(add);
        Ok(())
    }

    /// Claim accumulated dividends for the caller.
    pub fn claim_dividends(ctx: Context<Claim>) -> Result<()> {
        let b = &mut ctx.accounts.basket;
        let shares = ctx.accounts.user_basket_ata.amount as u128;
        let acc = b.acc_div_per_share_q64;
        let pending_q64 = shares.saturating_mul(acc).saturating_sub(ctx.accounts.user_position.div_debt_q64);
        let pending = (pending_q64 >> 64) as u64;
        if pending > 0 {
            let seeds = &[b"basket".as_ref(), b.basket_mint.as_ref(), &[b.bump]];
            let signer_seeds = &[&seeds[..]];
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.base_vault.to_account_info(),
                    to: ctx.accounts.user_base_ata.to_account_info(),
                    authority: ctx.accounts.basket.to_account_info(),
                },
            ).with_signer(signer_seeds);
            token::transfer(cpi_ctx, pending)?;
        }
        ctx.accounts.user_position.div_debt_q64 = shares.saturating_mul(acc);
        Ok(())
    }

    /// Set the paused flag; when paused mint/redeem operations revert.
    pub fn set_pause(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
        ctx.accounts.basket.paused = paused;
        Ok(())
    }
}

/// Context: create a new basket.
#[derive(Accounts)]
pub struct CreateBasket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// The basket PDA account; seeds set by `[b"basket", basket_mint.key().as_ref()]`.
    #[account(
        init,
        payer = admin,
        space = 8 + Basket::SIZE,
        seeds = [b"basket", basket_mint.key().as_ref()],
        bump
    )]
    pub basket: Account<'info, Basket>,
    #[account(mut)]
    pub base_mint: Account<'info, Mint>,
    #[account(mut)]
    pub basket_mint: Account<'info, Mint>,
    /// Vault to hold base tokens; must be associated with the basket PDA.
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

/// Context: Admin-only operations.
#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, has_one = admin)]
    pub basket: Account<'info, Basket>,
    #[account(mut)]
    pub base_mint: Account<'info, Mint>,
    #[account(mut)]
    pub basket_mint: Account<'info, Mint>,
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    /// Additional admin ATA for deposit_dividends.
    #[account(mut)]
    pub admin_base_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    /// The Token2022 program for mint/burn calls.
    pub token_2022_program: Program<'info, Token2022>,
}

/// Context: Mint shares.
#[derive(Accounts)]
pub struct MintShares<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub basket: Account<'info, Basket>,
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_base_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_basket_ata: Account<'info, TokenAccount>,
    pub base_mint: Account<'info, Mint>,
    #[account(mut)]
    pub basket_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + UserPosition::SIZE,
        seeds = [b"pos", basket.key().as_ref(), payer.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

/// Context: Redeem shares.
#[derive(Accounts)]
pub struct RedeemShares<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub basket: Account<'info, Basket>,
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_base_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_basket_ata: Account<'info, TokenAccount>,
    pub base_mint: Account<'info, Mint>,
    #[account(mut)]
    pub basket_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}

/// Context: Claim dividends.
#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub basket: Account<'info, Basket>,
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_base_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_basket_ata: Account<'info, TokenAccount>,
    pub base_mint: Account<'info, Mint>,
    pub basket_mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"pos", basket.key().as_ref(), payer.key().as_ref()], bump)]
    pub user_position: Account<'info, UserPosition>,
    pub token_program: Program<'info, Token>,
}

/// Basket account storing state for the basket token.
#[account]
pub struct Basket {
    pub admin: Pubkey,
    pub base_mint: Pubkey,
    pub basket_mint: Pubkey,
    pub base_vault: Pubkey,
    pub mgmt_fee_bps: u16,
    pub paused: bool,
    pub last_fee_ts: i64,
    pub nav_per_share_q64: u128,
    pub acc_div_per_share_q64: u128,
    pub bump: u8,
}
impl Basket {
    /// Size of the basket account data (without the anchor discriminator).
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 2 + 1 + 8 + 16 + 16 + 1;
}

/// Per-user position storing dividend debt.
#[account]
pub struct UserPosition {
    pub owner: Pubkey,
    pub basket: Pubkey,
    pub div_debt_q64: u128,
}
impl UserPosition {
    pub const SIZE: usize = 32 + 32 + 16;
}

/// Custom error codes.
#[error_code]
pub enum BasketError {
    #[msg("Paused")]
    Paused,
    #[msg("Slippage")]
    Slippage,
    #[msg("Oracle stale or NAV not set")]
    StaleOracle,
    #[msg("No supply")]
    NoSupply,
}

/// Accrue management fees (no-op in MVP).
fn accrue_fees(_b: &mut Basket) -> Result<()> {
    // For MVP, skip fee accrual. Production version would accrue mgmt fee over time.
    Ok(())
}
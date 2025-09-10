use anchor_lang::prelude::*;
use basket::program::Basket as BasketProgram;
use basket::Basket;

declare_id!("OraCLE1111111111111111111111111111111111");

#[program]
pub mod oracle_adapter {
    use super::*;
    /// Set the NAV per share for a basket. Only the admin may call this.
    pub fn set_nav(ctx: Context<SetNav>, nav_per_share_q64: u128) -> Result<()> {
        let basket = &mut ctx.accounts.basket;
        // Ensure caller is the admin of the basket.
        require!(ctx.accounts.admin.key() == basket.admin, OracleError::Unauthorized);
        basket.nav_per_share_q64 = nav_per_share_q64;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SetNav<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, has_one = admin)]
    pub basket: Account<'info, Basket>,
}

#[error_code]
pub enum OracleError {
    #[msg("Unauthorized")]
    Unauthorized,
}
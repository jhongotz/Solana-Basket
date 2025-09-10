use anchor_lang::prelude::*;

declare_id!("CompLY11111111111111111111111111111111111");

#[program]
pub mod compliance {
    use super::*;
    /// Set the KYC status for a user. Only the admin can set.
    pub fn set_kyc(ctx: Context<SetKyc>, user: Pubkey, allowed: bool) -> Result<()> {
        let record = &mut ctx.accounts.kyc_record;
        record.admin = ctx.accounts.admin.key();
        record.user = user;
        record.allowed = allowed;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SetKyc<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + KycRecord::SIZE,
        seeds = [b"kyc", user.key().as_ref()],
        bump
    )]
    pub kyc_record: Account<'info, KycRecord>,
    /// CHECK: Arbitrary user; only used as seed key.
    pub user: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct KycRecord {
    pub admin: Pubkey,
    pub user: Pubkey,
    pub allowed: bool,
}
impl KycRecord {
    pub const SIZE: usize = 32 + 32 + 1;
}
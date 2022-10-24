//! TestInit instruction handler

use {
    crate::{error::LaunchpadError, state::multisig::Multisig},
    anchor_lang::prelude::*,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct TestInit<'info> {
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    #[account(init, payer = upgrade_authority, space = Multisig::LEN, seeds = [b"multisig"], bump)]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(init, payer = upgrade_authority, space = 0, seeds = [b"transfer_authority"], bump)]
    pub transfer_authority: AccountInfo<'info>,

    system_program: Program<'info, System>,
    // remaining accounts: 1 to Multisig::MAX_SIGNERS admin signers (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TestInitParams {
    pub min_signatures: u8,
}

pub fn test_init(ctx: Context<TestInit>, params: &TestInitParams) -> Result<()> {
    if !cfg!(feature = "test") {
        return err!(LaunchpadError::InvalidEnvironment);
    }

    // initialize multisig, this will fail if account is already initialized
    let mut multisig = ctx.accounts.multisig.load_init()?;

    multisig.set_signers(ctx.remaining_accounts, params.min_signatures)?;

    // record multisig PDA bump
    multisig.bump = *ctx
        .bumps
        .get("multisig")
        .ok_or(ProgramError::InvalidSeeds)?;

    Ok(())
}

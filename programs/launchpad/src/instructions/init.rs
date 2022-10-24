//! Init instruction handler

use {
    crate::state::{launchpad::Launchpad, multisig::Multisig},
    anchor_lang::prelude::*,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    #[account(init, payer = upgrade_authority, space = Multisig::LEN, seeds = [b"multisig"], bump)]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(init, payer = upgrade_authority, space = 0, seeds = [b"transfer_authority"], bump)]
    pub transfer_authority: AccountInfo<'info>,

    #[account(init, payer = upgrade_authority, space = Launchpad::LEN, seeds = [b"launchpad"], bump)]
    pub launchpad: AccountLoader<'info, Launchpad>,

    #[account(constraint = launchpad_program.programdata_address()? == Some(launchpad_program_data.key()))]
    pub launchpad_program: Program<'info, Launchpad>,

    #[account(constraint = launchpad_program_data.upgrade_authority_address == Some(upgrade_authority.key()))]
    pub launchpad_program_data: Account<'info, ProgramData>,

    system_program: Program<'info, System>,
    // remaining accounts: 1 to Multisig::MAX_SIGNERS admin signers (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitParams {
    pub min_signatures: u8,
    pub allow_new_auctions: bool,
    pub allow_auction_updates: bool,
    pub allow_new_bids: bool,
    pub allow_withdrawals: bool,
    pub new_auction_fee: Fee,
    pub auction_update_fee: Fee,
    pub invalid_bid_fee: Fee,
    pub trade_fee: Fee,
}

pub fn init(ctx: Context<Init>, params: &InitParams) -> Result<()> {
    // initialize multisig, this will fail if account is already initialized
    let mut multisig = ctx.accounts.multisig.load_init()?;

    multisig.set_signers(ctx.remaining_accounts, params.min_signatures)?;

    // record multisig PDA bump
    multisig.bump = *ctx
        .bumps
        .get("multisig")
        .ok_or(ProgramError::InvalidSeeds)?;

    // record launchpad
    let launchpad = ctx.accounts.launchpad.as_mut();
    launchpad.allow_new_auctions = params.allow_new_auctions;
    launchpad.allow_auction_updates = params.allow_auction_updates;
    launchpad.allow_new_bids = params.allow_new_bids;
    launchpad.allow_withdrawals = params.allow_withdrawals;
    launchpad.fees.new_auction = params.new_auction_fee;
    launchpad.fees.auction_update = params.auction_update_fee;
    launchpad.fees.invalid_bid = params.invalid_bid_fee;
    launchpad.fees.trade = params.trade_fee;
    launchpad.collected_fees.new_auction_sol = 0;
    launchpad.collected_fees.auction_update_sol = 0;
    launchpad.collected_fees.invalid_bid_usdc = 0;
    launchpad.collected_fees.trade_usdc = 0;
    launchpad.transfer_authority_bump = *ctx
        .bumps
        .get("transfer_authority")
        .ok_or(ProgramError::InvalidSeeds)?;
    launchpad.launchpad_bump = *ctx
        .bumps
        .get("launchpad")
        .ok_or(ProgramError::InvalidSeeds)?;

    if !launchpad.validate() {
        err!(LaunchpadError::InvalidLaunchpadConfig)
    } else {
        Ok(0)
    }
}

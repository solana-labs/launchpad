//! WhitelistAdd instruction handler

use {
    crate::state::{self, auction::Auction},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct WhitelistAdd<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    system_program: Program<'info, System>,
    // remaining accounts:
    //   Bid accounts for addresses to be whitelisted (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistAddParams {
    addresses: Vec<Pubkey>,
}

pub fn whitelist_add<'info>(
    ctx: Context<'_, '_, '_, 'info, WhitelistAdd<'info>>,
    params: &WhitelistAddParams,
) -> Result<()> {
    if ctx.remaining_accounts.is_empty() || ctx.remaining_accounts.len() != params.addresses.len() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    // load or initialize bid accounts
    let mut bid_accounts = state::create_bid_accounts(
        ctx.remaining_accounts,
        &params.addresses,
        ctx.accounts.owner.to_account_info(),
        &ctx.accounts.auction.key(),
        ctx.accounts.system_program.to_account_info(),
    )?;

    // add to white-list
    for bid in bid_accounts.iter_mut() {
        bid.whitelisted = true;
    }

    state::save_accounts(&bid_accounts)?;

    Ok(())
}

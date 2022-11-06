//! WhitelistRemove instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{self, auction::Auction, bid::Bid},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct WhitelistRemove<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
    // remaining accounts:
    //   Bid accounts to be removed from the whitelist (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistRemoveParams {}

pub fn whitelist_remove(
    ctx: Context<WhitelistRemove>,
    _params: &WhitelistRemoveParams,
) -> Result<()> {
    if ctx.remaining_accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    let mut bid_accounts = state::load_accounts::<Bid>(ctx.remaining_accounts, &crate::ID)?;
    for bid in &mut bid_accounts {
        // TODO validate address
        bid.whitelisted = false;
    }
    // TODO if auction is ended close accounts instead
    state::save_accounts(&bid_accounts)?;

    Ok(())
}

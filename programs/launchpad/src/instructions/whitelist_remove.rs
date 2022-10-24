//! WhitelistRemove instruction handler

use {
    crate::{error::LaunchpadError},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct WhitelistRemove<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    system_program: Program<'info, System>,

    // remaining accounts:
    //   Bid accounts to be removed from the whitelist (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistRemoveParams {
}

pub fn whitelist_remove(
    ctx: Context<WhitelistRemove>,
    params: &WhitelistRemoveParams,
) -> Result<()> {
    let bid_accounts = load_accounts::<Bid>(ctx.remaining_accounts)?;
    for bid in bid_accounts {
        // TODO validate address
        bid.whitelisted = false;
    }
    // TODO if auction is ended close accounts instead
    save_accounts(&bid_accounts)?;

    Ok(())
}

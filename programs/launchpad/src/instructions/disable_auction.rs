//! DisableAuction instruction handler

use {
    crate::{error::LaunchpadError},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct DisableAuction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DisableAuctionParams {
}

pub fn disable_auction(
    ctx: Context<DisableAuction>,
    params: &DisableAuctionParams,
) -> Result {
    let auction = ctx.accounts.auction.as_mut();
    auction.enabled = false;

    Ok(())
}

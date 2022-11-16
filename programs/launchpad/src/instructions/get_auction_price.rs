//! GetAuctionPrice instruction handler

use {
    crate::state::{auction::Auction, launchpad::Launchpad},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetAuctionPrice<'info> {
    #[account()]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"launchpad"], 
        bump = launchpad.launchpad_bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetAuctionPriceParams {
    amount: u64,
}

pub fn get_auction_price(
    ctx: Context<GetAuctionPrice>,
    params: &GetAuctionPriceParams,
) -> Result<u64> {
    ctx.accounts
        .auction
        .get_auction_price(params.amount, ctx.accounts.auction.get_time()?)
}

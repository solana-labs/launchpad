//! GetAuctionPrice instruction handler

use {crate::state::auction::Auction, anchor_lang::prelude::*};

#[derive(Accounts)]
pub struct GetAuctionPrice<'info> {
    #[account()]
    pub user: Signer<'info>,

    #[account(seeds = [b"launchpad"], bump = launchpad.bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        seeds = [b"auction", auction.name.as_bytes()],
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
    _params: &GetAuctionPriceParams,
) -> Result<u64> {
    Ok(ctx.auction.get_auction_price(amount)?)
}

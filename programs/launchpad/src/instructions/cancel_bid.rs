//! CancelBid instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{auction::Auction, bid::Bid},
    },
    anchor_lang::{prelude::*, AccountsClose},
};

#[derive(Accounts)]
pub struct CancelBid<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(
        seeds = [b"auction",
                 auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        mut,
        seeds = [b"bid",
                 bid.owner.key().as_ref(),
                 auction.key().as_ref()],
        bump = bid.bump
    )]
    pub bid: Box<Account<'info, Bid>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelBidParams {}

pub fn cancel_bid(ctx: Context<CancelBid>, _params: &CancelBidParams) -> Result<()> {
    require!(
        ctx.accounts
            .auction
            .is_ended(ctx.accounts.auction.get_time()?, true),
        LaunchpadError::AuctionInProgress
    );

    let bid = ctx.accounts.bid.as_mut();
    if (!bid.seller_initialized && ctx.accounts.initializer.key() == bid.owner)
        || (bid.seller_initialized && ctx.accounts.initializer.key() == ctx.accounts.auction.owner)
    {
        bid.close(ctx.accounts.initializer.to_account_info())
    } else {
        Err(ProgramError::IllegalOwner.into())
    }
}

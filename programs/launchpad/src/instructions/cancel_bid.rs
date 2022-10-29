//! CancelBid instruction handler

use {
    crate::state::{auction::Auction, bid::Bid},
    anchor_lang::{prelude::*, AccountsClose},
};

#[derive(Accounts)]
pub struct CancelBid<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"bid", auction.key().as_ref()],
        bump = bid.bump
    )]
    pub bid: Box<Account<'info, Bid>>,

    #[account(
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelBidParams {
}

pub fn cancel_bid(ctx: Context<CancelBid>, _params: &CancelBidParams) -> Result<()> {
    if ctx.accounts.auction.is_ended(ctx.accounts.auction.get_time()?) {
        ctx.accounts
                .bid
                .close(ctx.accounts.owner.to_account_info())?;
    }

    Ok(())
}

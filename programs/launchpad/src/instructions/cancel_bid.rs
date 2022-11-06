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
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        mut,
        seeds = [b"bid", auction.key().as_ref()],
        bump = bid.bump
    )]
    pub bid: Box<Account<'info, Bid>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelBidParams {}

pub fn cancel_bid(ctx: Context<CancelBid>, _params: &CancelBidParams) -> Result<()> {
    if ctx
        .accounts
        .auction
        .is_ended(ctx.accounts.auction.get_time()?)
    {
        let bid = ctx.accounts.bid.as_mut();
        if ctx.accounts.owner.key() == bid.owner
            || (bid.seller_initialized && ctx.accounts.auction.owner == bid.owner)
        {
            bid.close(ctx.accounts.owner.to_account_info())?;
        } else {
            return Err(ProgramError::IllegalOwner.into());
        }
    }

    Ok(())
}

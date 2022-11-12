//! EnableAuction instruction handler

use {
    crate::state::auction::Auction,
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct EnableAuction<'info> {
    #[account()]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EnableAuctionParams {
}

pub fn enable_auction(
    ctx: Context<EnableAuction>,
    _params: &EnableAuctionParams,
) -> Result<()> {
     let auction = ctx.accounts.auction.as_mut();
     auction.enabled = true;

    Ok(())
}

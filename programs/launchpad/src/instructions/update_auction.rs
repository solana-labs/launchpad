//! UpdateAuction instruction handler

use {
    crate::{error::LaunchpadError},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct UpdateAuction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [b"launchpad"], bump = launchpad.bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateAuctionParams {
    pub common: CommonParams,
    pub payment: PaymentParams,
    pub pricing: PricingParams,
    pub token_ratios: Vec<u64>,
}

pub fn update_auction(
    ctx: Context<UpdateAuction>,
    params: &UpdateAuctionParams,
) -> Result {
    require!(ctx.accounts.launchpad.allow_auction_updates, 
        LaunchpadError::AuctionUpdatesNotAllowed);

    // update auction data
    let auction = ctx.accounts.auction.as_mut();

    if !auction.updatable {
        return err!(LaunchpadError::AuctionNotUpdatable);
    }

    auction.common = params.common;
    auction.payment = params.payment;
    auction.pricing = params.pricing;
    auction.stats = AuctionStats::default();

    for n in 0..auction.num_tokens {
        auction.tokens[n].ratio = params.token_ratios[n];
    }

    if !auction.validate() {
        err!(LaunchpadError::InvalidAuctionConfig)
    } else {
        Ok(0)
    }

    Ok(())
}

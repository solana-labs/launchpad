//! UpdateAuction instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{
            auction::{Auction, AuctionStats, CommonParams, PaymentParams, PricingParams},
            launchpad::Launchpad,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct UpdateAuction<'info> {
    #[account()]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"launchpad"],
        bump = launchpad.launchpad_bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut,
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
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

pub fn update_auction(ctx: Context<UpdateAuction>, params: &UpdateAuctionParams) -> Result<()> {
    require!(
        ctx.accounts.launchpad.permissions.allow_auction_updates,
        LaunchpadError::AuctionUpdatesNotAllowed
    );

    // update auction data
    let auction = ctx.accounts.auction.as_mut();

    require!(auction.updatable, LaunchpadError::AuctionNotUpdatable);

    auction.common = params.common.clone();
    auction.payment = params.payment;
    auction.pricing = params.pricing;

    for n in 0..(auction.num_tokens as usize) {
        auction.tokens[n].ratio = params.token_ratios[n];
    }

    auction.update_time = auction.get_time()?;

    if !auction.validate()? {
        err!(LaunchpadError::InvalidAuctionConfig)
    } else {
        Ok(())
    }
}

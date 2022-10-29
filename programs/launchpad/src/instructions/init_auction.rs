//! InitAuction instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{
            self,
            auction::{
                Auction, AuctionStats, AuctionToken, CommonParams, PaymentParams, PricingParams,
            },
            custody::Custody,
            launchpad::Launchpad,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
#[instruction(params: InitAuctionParams)]
pub struct InitAuction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [b"launchpad"], bump = launchpad.launchpad_bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(init,
              payer = owner,
              space = Auction::LEN,
              seeds = [b"auction", params.common.name.as_bytes()],
              bump)]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        seeds = [b"custody", pricing_custody.mint.as_ref()],
        bump = pricing_custody.bump
    )]
    pub pricing_custody: Box<Account<'info, Custody>>,

    system_program: Program<'info, System>,
    // remaining accounts:
    //   1 to Auction::MAX_TOKENS dispensing custody addresses (write, unsigned)
    //      with seeds = [b"dispense", mint.key().as_ref(), auction.key().as_ref()],
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitAuctionParams {
    pub enabled: bool,
    pub updatable: bool,
    pub common: CommonParams,
    pub payment: PaymentParams,
    pub pricing: PricingParams,
    pub token_ratios: Vec<u64>,
}

pub fn init_auction(ctx: Context<InitAuction>, params: &InitAuctionParams) -> Result<()> {
    require!(
        ctx.accounts.launchpad.permissions.allow_new_auctions,
        LaunchpadError::NewAuctionsNotAllowed
    );

    // create dispensing accounts
    // TODO check addresses
    let dispensers = state::create_token_accounts(
        ctx.remaining_accounts,
        &ctx.accounts.owner.key(),
        Auction::MAX_TOKENS,
    )?;
    state::save_accounts(&dispensers)?;

    require_keys_eq!(
        ctx.accounts.pricing_custody.key(),
        params.pricing.custody,
        LaunchpadError::InvalidPricingConfig
    );

    // record auction data
    let auction = ctx.accounts.auction.as_mut();

    auction.owner = ctx.accounts.owner.key();
    auction.enabled = params.enabled;
    auction.updatable = params.updatable;
    auction.common = params.common.clone();
    auction.payment = params.payment;
    auction.pricing = params.pricing;
    auction.stats = AuctionStats::default();
    auction.stats.wl_bidders.min_fill_price = u64::MAX;
    auction.stats.reg_bidders.min_fill_price = u64::MAX;
    auction.tokens = [AuctionToken::default(); Auction::MAX_TOKENS];
    auction.num_tokens = dispensers.len() as u8;

    for n in 0..(auction.num_tokens as usize) {
        auction.tokens[n].ratio = params.token_ratios[n];
        auction.tokens[n].account = dispensers[n].key();
    }

    auction.bump = *ctx.bumps.get("auction").ok_or(ProgramError::InvalidSeeds)?;

    auction.creation_time = if cfg!(feature = "test") {
        0
    } else {
        auction.get_time()?
    };
    auction.update_time = auction.creation_time;

    if !auction.validate(auction.get_time()?) {
        err!(LaunchpadError::InvalidAuctionConfig)
    } else {
        Ok(())
    }
}

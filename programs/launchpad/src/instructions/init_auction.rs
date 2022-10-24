//! InitAuction instruction handler

use {crate::error::LaunchpadError, anchor_lang::prelude::*};

#[derive(Accounts)]
#[instruction(params: InitAuctionParams)]
pub struct InitAuction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(init,
              payer = owner,
              space = Auction::LEN,
              seeds = [b"auction", params.name.as_bytes()],
              bump)]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        constraint = pricing_custody.key() == auction.pricing.custody,
        seeds = [b"custody", pricing_custody.mint.as_ref()],
        bump = pricing_custody.bump
    )]
    pub pricing_custody: Box<Account<'info, Custody>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
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

pub fn init_auction(ctx: Context<InitAuction>, params: &InitAuctionParams) -> Result<u8> {
    require!(
        ctx.accounts.launchpad.allow_new_auctions,
        LaunchpadError::NewAuctionsNotAllowed
    );

    // load dispensing accounts
    let dispensers = load_dispensing_custodies()?;

    if ctx.accounts.pricing_custody.key() != params.pricing.custody {
        return err!(LaunchpdaError::InvalidPricingConfig);
    }

    // record auction data
    let auction = ctx.accounts.auction.as_mut();

    auction.owner = ctx.accounts.owner.key();
    auction.enabled = params.enabled;
    auction.updatable = params.updatable;
    auction.common = params.common;
    auction.payment = params.payment;
    auction.pricing = params.pricing;
    auction.stats = AuctionStats::default();
    auction.stats.wl_bidders.min_fill_price = u64::MAX;
    auction.stats.reg_bidders.min_fill_price = u64::MAX;
    auction.tokens = [AuctionToken::default(); Auction::MAX_TOKENS];
    auction.num_tokens = dispensers.len();

    for n in 0..auction.num_tokens {
        auction.tokens[n].ratio = params.token_ratios[n];
        auction.tokens[n].account = dispensers[n].key();
    }

    auction.bump = *ctx.bumps.get("auction").ok_or(ProgramError::InvalidSeeds)?;

    auction.creation_time = if cfg!(feature = "test") {
        0
    } else {
        auction.get_time()?
    };

    if !auction.validate() {
        err!(LaunchpadError::InvalidAuctionConfig)
    } else {
        Ok(0)
    }
}

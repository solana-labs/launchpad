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
    anchor_spl::token::Token,
};

#[derive(Accounts)]
#[instruction(params: InitAuctionParams)]
pub struct InitAuction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        mut,
        seeds = [b"transfer_authority"],
        bump = launchpad.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"launchpad"],
        bump = launchpad.launchpad_bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        init,
        payer = owner,
        space = Auction::LEN,
        seeds = [b"auction", params.common.name.as_bytes()],
        bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        seeds = [b"custody", pricing_custody.mint.as_ref()],
        bump = pricing_custody.bump
    )]
    pub pricing_custody: Box<Account<'info, Custody>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
    // remaining accounts:
    //   1 to Auction::MAX_TOKENS dispensing custody addresses (write, unsigned)
    //      with seeds = [b"dispense", mint.key().as_ref(), auction.key().as_ref()],
    //   1 to Auction::MAX_TOKENS dispensing custody mints (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitAuctionParams {
    pub enabled: bool,
    pub updatable: bool,
    pub fixed_amount: bool,
    pub common: CommonParams,
    pub payment: PaymentParams,
    pub pricing: PricingParams,
    pub token_ratios: Vec<u64>,
}

pub fn init_auction<'info>(
    ctx: Context<'_, '_, '_, 'info, InitAuction<'info>>,
    params: &InitAuctionParams,
) -> Result<()> {
    require!(
        ctx.accounts.launchpad.permissions.allow_new_auctions,
        LaunchpadError::NewAuctionsNotAllowed
    );

    // collect fee
    let launchpad = ctx.accounts.launchpad.as_mut();
    state::transfer_sol(
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        launchpad.fees.new_auction,
    )?;
    launchpad.collected_fees.new_auction_sol = launchpad
        .collected_fees
        .new_auction_sol
        .wrapping_add(launchpad.fees.new_auction);

    // create dispensing accounts
    if ctx.remaining_accounts.is_empty() || ctx.remaining_accounts.len() % 2 != 0 {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }
    let accounts_half_len = ctx.remaining_accounts.len() / 2;
    require!(
        accounts_half_len <= Auction::MAX_TOKENS,
        LaunchpadError::TooManyAccountKeys
    );
    let dispensers = state::create_token_accounts(
        &ctx.remaining_accounts[..accounts_half_len],
        &ctx.remaining_accounts[accounts_half_len..],
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        &ctx.accounts.auction.key(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.rent.to_account_info(),
    )?;

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
    auction.fixed_amount = params.fixed_amount;
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

    if !auction.validate()? {
        err!(LaunchpadError::InvalidAuctionConfig)
    } else {
        Ok(())
    }
}

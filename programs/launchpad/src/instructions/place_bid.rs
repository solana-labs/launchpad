//! PlaceBid instruction handler

use {
    crate::{
        error::LaunchpadError,
        math,
        state::{
            auction::Auction,
        },
    },
    anchor_lang::{prelude::*, AccountsClose},
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        constraint = funding_account.mint == payment_custody.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        mut, seeds = [b"transfer_authority"], 
        bump = auction.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(mut, seeds = [b"launchpad"], bump = launchpad.bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut, 
        constraint = auction.owner == seller_balance.owner,
        seeds = [b"auction", auction.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        init_if_needed, 
        payer = owner, 
        space = SellerBalance::LEN, 
        constraint = seller_balance.custody == payment_custody.key(),
        seeds = [b"seller_balance", seller_balance.custody.as_ref()],
        bump = seller_balance.bump
    )]
    pub seller_balance: Box<Account<'info, SellerBalance>>,

    #[account(
        init_if_needed, 
        payer = owner, 
        space = Bid::LEN, 
        has_one = owner,
        seeds = [b"bid", owner.key().as_ref(), auction.key().as_ref()],
        bump
    )]
    pub bid: Box<Account<'info, Bid>>,

    #[account(
        mut,
        constraint = pricing_custody.key() == auction.pricing.custody,
        seeds = [b"custody", pricing_custody.mint.as_ref()],
        bump = pricing_custody.bump
    )]
    pub pricing_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the pricing token
    #[account(constraint = pricing_oracle_account.key() == pricing_custody.oracle_account)]
    pub pricing_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody", payment_custody.mint.as_ref()],
        bump = payment_custody.bump
    )]
    pub payment_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the payment token
    #[account(constraint = payment_oracle_account.key() == payment_custody.oracle_account)]
    pub payment_oracle_account: AccountInfo<'info>,

    /// CHECK: account constraints checked in account trait
	#[account(address = sysvar::slot_hashes::id())]
	recent_slothashes: UncheckedAccount<'info>,
    
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,

    // remaining accounts:
    //   1 to Auction::MAX_TOKENS user's token receiving accounts (write, unsigned)
    //   1 to Auction::MAX_TOKENS dispensing custody addresses (write, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PlaceBidParams {
    price: u64,
    amount: u64,
    bid_type: BidType
}

pub fn place_bid(ctx: Context<PlaceBid>, params: &PlaceBidParams) -> Result<()> {
    require!(ctx.accounts.launchpad.allow_new_bids, 
        LaunchpadError::BidsNotAllowed);

    // validate inputs
    require_gt!(params.lp_amount, 0u64, LaunchpadError::InvalidTokenAmount);

    // load accounts
    if ctx.remaining_accounts.is_empty() || ctx.remaining_accounts.len() % 2 != 0 {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }
    let accounts_len = ctx.remaining_accounts.len();
    let receiving_accounts = 
        load_accounts::<TokenAccount>(ctx.remaining_accounts[0..accounts_len/2])?;
    let dispensing_custodies = 
        load_accounts::<TokenAccount>(ctx.remaining_accounts[accounts_len/2..accounts_len.len()])?;
    
    // get the available amount at the given price
    let auction = ctx.accounts.auction.as_mut();
    let avail_amount = auction.get_auction_amount(params.price)?;

    if avail_amount == 0 || (params.bid_type == BidType::FOK && 
        avail_amount < params.amount) {
        return err!(LaunchpadError::InsufficientAmount);
    }
    let fill_amount = std::cmp::min(avail_amount, params.amount);

    let bid = ctx.accounts.bid.as_mut();
    if bid.bump == 0 {
        bid.owner = owner;
        bid.auction = ctx.accounts.auction.key();
        bid.whitelisted = false;
        bid.seller_initialized = false;
        bid.bump = *ctx.bumps.get("bid").ok_or(ProgramError::InvalidSeeds)?;
    }
    
    bid.bid_time = auction.get_time()?;
    bid.bid_price = params.price;
    bid.bid_amount = params.amount;
    bid.bid_type = params.bid_type;
    bid.filled = math::checked_add(bid.filled, fill_amount)?;
    bid.fill_time = bid.bid_time;

    // update seller's balance
    if seller_balance.bump == 0 {
        seller_balance.owner = auction.owner;
        seller_balance.custody = ctx.accounts.payment_custody.key();
        seller_balance.bump = 
            *ctx.bumps.get("seller_balance").ok_or(ProgramError::InvalidSeeds)?;
    }
    seller_balance.balance = 
        math::checked_add(seller_balance.balance, pay_amount)?;

    // update auction stats
    let curtime = auction.get_time()?;
    if auction.stats.first_trade_time == 0 {
        auction.stats.first_trade_time = curtime;
    }
    auction.stats.last_trade_time = curtime;
    auction.stats.last_amount = fill_amount;
    auction.stats.last_price = params.price;

    let bidder_stats = if bid.whitelisted {
        &auction.stats.wl_bidders
    } else {
        &auction.stats.reg_bidders
    };
    bidder_stats.fills_volume = math::checked_add(bidder_stats.fills_volume, fill_volume)?;
    bidder_stats.weighted_fills_sum = math::checked_add(bidder_stats.weighted_fills_sum as u128, 
        math::checked_mul(fill_volume as u128, params.price as u128)?)?;
    if params.price < bidder_stats.min_fill_price {
        bidder_stats.min_fill_price = params.price;
    }
    if params.price > bidder_stats.max_fill_price {
        bidder_stats.max_fill_price = params.price;
    }
    bidder_stats.num_trades = math::checked_add(bidder_stats.num_trades, 1)?;

    // collect payment
    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.payment_custody.to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        params.price,
    )?;

    // pick a random token and send it
	let slothashes_data = ctx.accounts.recent_slothashes.data.borrow();
    if (slothashes_data.len() < 20) {
        return Err(ProgramError::InvalidAccountData.into());
    }
	let rand_seed = u64::from_le_bytes(slothashes_data[12..20].try_into().unwrap());
    let token_num = rand_seed % dispensing_custodies.len();

    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.dispensing_custodies[token_num].to_account_info(),
        ctx.accounts.receiving_account[token_num].to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        fill_amount,
    )?;

    Ok(())
}

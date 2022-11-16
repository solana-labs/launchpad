//! PlaceBid instruction handler

use {
    crate::{
        error::LaunchpadError,
        math,
        state::{
            self,
            auction::Auction,
            bid::{BadBidType, Bid, BidType},
            custody::Custody,
            launchpad::Launchpad,
            oracle::OraclePrice,
            seller_balance::SellerBalance,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount, Transfer},
    solana_program::sysvar,
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
        mut,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        init_if_needed,
        payer = owner,
        space = SellerBalance::LEN,
        seeds = [b"seller_balance", auction.owner.as_ref(), payment_custody.key().as_ref()],
        bump
    )]
    pub seller_balance: Box<Account<'info, SellerBalance>>,

    #[account(
        init_if_needed,
        payer = owner,
        space = Bid::LEN,
        seeds = [b"bid", owner.key().as_ref(), auction.key().as_ref()],
        bump
    )]
    pub bid: Box<Account<'info, Bid>>,

    #[account(
        constraint = pricing_custody.key() == auction.pricing.custody,
        seeds = [b"custody", pricing_custody.mint.as_ref()],
        bump = pricing_custody.bump
    )]
    pub pricing_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the pricing token
    #[account(
        constraint = pricing_oracle_account.key() == pricing_custody.oracle_account
    )]
    pub pricing_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody", payment_custody.mint.as_ref()],
        bump = payment_custody.bump
    )]
    pub payment_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the payment token
    #[account(
        constraint = payment_oracle_account.key() == payment_custody.oracle_account
    )]
    pub payment_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        constraint = payment_token_account.key() == payment_custody.token_account.key()
    )]
    pub payment_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: account constraints checked in account trait
    #[account(
        address = sysvar::slot_hashes::id()
    )]
    recent_slothashes: UncheckedAccount<'info>,

    /// CHECK: account constraints checked in account trait
    #[account(
        address = sysvar::instructions::id()
    )]
    instructions: UncheckedAccount<'info>,

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
    bid_type: BidType,
}

pub fn place_bid<'info>(
    ctx: Context<'_, '_, '_, 'info, PlaceBid<'info>>,
    params: &PlaceBidParams,
) -> Result<()> {
    require!(
        ctx.accounts.launchpad.permissions.allow_new_bids,
        LaunchpadError::BidsNotAllowed
    );

    // check if this instruction is the only instruction in the transaction
    require!(
        sysvar::instructions::load_current_index_checked(
            &ctx.accounts.instructions.to_account_info()
        )? == 0
            && sysvar::instructions::load_instruction_at_checked(
                1,
                &ctx.accounts.instructions.to_account_info()
            )
            .is_err(),
        LaunchpadError::MustBeSingleInstruction
    );

    // load accounts
    msg!("Load accounts");
    let launchpad = ctx.accounts.launchpad.as_mut();
    let auction = ctx.accounts.auction.as_mut();
    let bid = ctx.accounts.bid.as_mut();
    let seller_balance = ctx.accounts.seller_balance.as_mut();
    let payment_custody = ctx.accounts.payment_custody.as_mut();

    if ctx.remaining_accounts.is_empty() || ctx.remaining_accounts.len() % 2 != 0 {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }
    let accounts_half_len = ctx.remaining_accounts.len() / 2;
    if accounts_half_len > auction.num_tokens.into() {
        return err!(LaunchpadError::TooManyAccountKeys);
    }
    if accounts_half_len < auction.num_tokens.into() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }
    let receiving_accounts = state::load_accounts::<TokenAccount>(
        &ctx.remaining_accounts[..accounts_half_len],
        &Token::id(),
    )?;
    let dispensing_custodies = state::load_accounts::<TokenAccount>(
        &ctx.remaining_accounts[accounts_half_len..],
        &Token::id(),
    )?;

    // validate inputs
    msg!("Validate inputs");
    require_gt!(params.amount, 0u64, LaunchpadError::InvalidTokenAmount);
    let order_amount_limit = if bid.whitelisted {
        std::cmp::max(
            auction.common.order_limit_wl_address,
            auction.common.order_limit_reg_address,
        )
    } else {
        auction.common.order_limit_reg_address
    };
    require_gte!(
        order_amount_limit,
        params.amount,
        LaunchpadError::BidAmountTooLarge
    );
    require_gte!(
        params.price,
        auction.pricing.min_price,
        LaunchpadError::BidPriceTooSmall
    );

    // check if auction is active
    let curtime = auction.get_time()?;
    let mut bad_bid_type = BadBidType::None;

    if !auction.is_started(curtime, bid.whitelisted) {
        bad_bid_type = BadBidType::TooEarly;
    }

    require!(
        !auction.is_ended(curtime, bid.whitelisted),
        LaunchpadError::AuctionEnded
    );

    // validate dispensing and receiving accounts
    // all accounts needs to be validated, not the only selected to dispense,
    // so the user can't game the process
    msg!("Validate dispensing and receiving accounts");
    for token in 0..auction.num_tokens as usize {
        if receiving_accounts[token].owner != ctx.accounts.owner.key() {
            msg!("Invalid owner of the receiving token account");
            return Err(ProgramError::IllegalOwner.into());
        }
        require_keys_eq!(
            dispensing_custodies[token].key(),
            auction.tokens[token].account,
            LaunchpadError::InvalidDispenserAddress
        );
        require_keys_eq!(
            dispensing_custodies[token].mint,
            receiving_accounts[token].mint,
            LaunchpadError::InvalidReceivingAddress
        )
    }

    // pick a random token to dispense
    msg!("Select token to dispense");
    let token_num = if auction.num_tokens == 1 {
        0
    } else {
        let slothashes_data = ctx.accounts.recent_slothashes.data.borrow();
        if slothashes_data.len() < 20 {
            return Err(ProgramError::InvalidAccountData.into());
        }
        let rand_seed = usize::from_le_bytes(slothashes_data[12..20].try_into().unwrap());
        rand_seed % dispensing_custodies.len()
    };
    let max_amount_to_dispense = math::checked_div(
        dispensing_custodies[token_num].amount,
        auction.pricing.unit_size,
    )?;

    // get available amount at the given price
    msg!("Compute available amount");
    let avail_amount = std::cmp::min(
        auction.get_auction_amount(params.price, curtime)?,
        max_amount_to_dispense,
    );

    if avail_amount == 0 || (params.bid_type == BidType::Fok && avail_amount < params.amount) {
        return err!(LaunchpadError::InsufficientAmount);
    }
    let fill_amount = std::cmp::min(avail_amount, params.amount);

    let fill_price = auction.get_auction_price(fill_amount, curtime)?;
    require_gte!(params.price, fill_price, LaunchpadError::PriceCalcError);

    // check for malicious bid
    let fill_amount_limit = if bid.whitelisted {
        std::cmp::max(
            auction.common.fill_limit_wl_address,
            auction.common.fill_limit_reg_address,
        )
    } else {
        auction.common.fill_limit_reg_address
    };
    if fill_amount_limit < bid.filled {
        bad_bid_type = BadBidType::FillLimit;
    }

    if bad_bid_type != BadBidType::None {
        if launchpad.fees.invalid_bid.is_zero() {
            if bad_bid_type == BadBidType::TooEarly {
                return err!(LaunchpadError::AuctionNotStarted);
            } else {
                return err!(LaunchpadError::FillAmountLimit);
            }
        } else {
            return collect_bad_bid_fee(
                launchpad,
                payment_custody,
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.funding_account.to_account_info(),
                ctx.accounts.payment_token_account.to_account_info(),
                ctx.accounts.pricing_oracle_account.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                std::cmp::min(fill_amount, ctx.accounts.funding_account.amount),
                curtime,
            );
        }
    }

    // compute payment amount
    let mut payment_amount = 0;
    if fill_price > 0 {
        msg!("Compute payment amount");
        let payment_token_price = if !launchpad.fees.trade.is_zero()
            || payment_custody.key() != ctx.accounts.pricing_custody.key()
        {
            OraclePrice::new_from_oracle(
                payment_custody.oracle_type,
                &ctx.accounts.payment_oracle_account.to_account_info(),
                payment_custody.max_oracle_price_error,
                payment_custody.max_oracle_price_age_sec,
                curtime,
            )?
        } else {
            OraclePrice::new(0, 0)
        };

        if payment_custody.key() == ctx.accounts.pricing_custody.key() {
            payment_amount = math::checked_mul(fill_price, fill_amount)?;
        } else {
            let pricing_custody = &ctx.accounts.pricing_custody;
            let auction_token_price = OraclePrice::new_from_oracle(
                pricing_custody.oracle_type,
                &ctx.accounts.pricing_oracle_account.to_account_info(),
                pricing_custody.max_oracle_price_error,
                pricing_custody.max_oracle_price_age_sec,
                curtime,
            )?;

            let token_pair_price = auction_token_price.checked_div(&payment_token_price)?;
            let price_per_token = math::checked_decimal_ceil_mul(
                fill_price,
                -(pricing_custody.decimals as i32),
                token_pair_price.price,
                token_pair_price.exponent,
                -(payment_custody.decimals as i32),
            )?;

            payment_amount = math::checked_mul(price_per_token, fill_amount)?;
        }

        // compute fee
        let fee_amount = launchpad.fees.trade.get_fee_amount(payment_amount)?;

        // collect payment and fee
        msg!("Collect payment {} and fee {}", payment_amount, fee_amount);
        let total_amount = math::checked_add(payment_amount, fee_amount)?;
        let context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.funding_account.to_account_info(),
                to: ctx.accounts.payment_token_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        );
        anchor_spl::token::transfer(context, total_amount)?;

        if fee_amount > 0 {
            payment_custody.collected_fees =
                math::checked_add(payment_custody.collected_fees, fee_amount)?;

            let fees_in_usdc = math::to_token_amount(
                payment_token_price.get_asset_value_usd(fee_amount, payment_custody.decimals)?,
                6,
            )?;

            launchpad.collected_fees.trade_usdc = launchpad
                .collected_fees
                .trade_usdc
                .wrapping_add(fees_in_usdc);
        }
    }

    // update user's bid
    msg!("Update user's bid");
    if bid.bump == 0 {
        bid.owner = ctx.accounts.owner.key();
        bid.auction = auction.key();
        bid.whitelisted = false;
        bid.seller_initialized = false;
        bid.bump = *ctx.bumps.get("bid").ok_or(ProgramError::InvalidSeeds)?;
    } else if bid.owner != ctx.accounts.owner.key() || bid.auction != auction.key() {
        return err!(LaunchpadError::InvalidBidAddress);
    }

    bid.bid_time = auction.get_time()?;
    bid.bid_price = params.price;
    bid.bid_amount = params.amount;
    bid.bid_type = params.bid_type;
    bid.filled = math::checked_add(bid.filled, fill_amount)?;
    bid.fill_time = bid.bid_time;
    bid.fill_price = fill_price;
    bid.fill_amount = fill_amount;

    // update seller's balance
    msg!("Update seller's balance");
    if seller_balance.bump == 0 {
        seller_balance.owner = auction.owner;
        seller_balance.custody = ctx.accounts.payment_custody.key();
        seller_balance.bump = *ctx
            .bumps
            .get("seller_balance")
            .ok_or(ProgramError::InvalidSeeds)?;
    } else if seller_balance.owner != auction.owner
        || seller_balance.custody == ctx.accounts.payment_custody.key()
    {
        return err!(LaunchpadError::InvalidSellerBalanceAddress);
    }
    seller_balance.balance = math::checked_add(seller_balance.balance, payment_amount)?;

    // update auction stats
    msg!("Update auction stats");
    let curtime = auction.get_time()?;
    if auction.stats.first_trade_time == 0 {
        auction.stats.first_trade_time = curtime;
    }
    auction.stats.last_trade_time = curtime;
    auction.stats.last_amount = fill_amount;
    auction.stats.last_price = fill_price;

    let bidder_stats = if bid.whitelisted {
        &mut auction.stats.wl_bidders
    } else {
        &mut auction.stats.reg_bidders
    };
    bidder_stats.fills_volume = math::checked_add(bidder_stats.fills_volume, fill_amount)?;
    bidder_stats.weighted_fills_sum = math::checked_add(
        bidder_stats.weighted_fills_sum,
        math::checked_mul(fill_amount as u128, fill_price as u128)?,
    )?;
    if fill_price < bidder_stats.min_fill_price {
        bidder_stats.min_fill_price = fill_price;
    }
    if fill_price > bidder_stats.max_fill_price {
        bidder_stats.max_fill_price = fill_price;
    }
    bidder_stats.num_trades = bidder_stats.num_trades.wrapping_add(1);

    // transfer purchased tokens to the user
    let transfer_amount = math::checked_mul(fill_amount, auction.pricing.unit_size)?;
    msg!("Transfer {} tokens to the user", transfer_amount);
    ctx.accounts.launchpad.transfer_tokens(
        dispensing_custodies[token_num].to_account_info(),
        receiving_accounts[token_num].to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        transfer_amount,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_bad_bid_fee<'info>(
    launchpad: &mut Account<'info, Launchpad>,
    custody: &mut Account<'info, Custody>,
    token_program: AccountInfo<'info>,
    funding_account: AccountInfo<'info>,
    destination_account: AccountInfo<'info>,
    oracle_account: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    bid_amount: u64,
    curtime: i64,
) -> Result<()> {
    let fee_amount = launchpad.fees.invalid_bid.get_fee_amount(bid_amount)?;
    if fee_amount == 0 {
        return Ok(());
    }

    msg!("Collect bad bid fee {}", fee_amount);
    let context = CpiContext::new(
        token_program,
        Transfer {
            from: funding_account,
            to: destination_account,
            authority,
        },
    );
    anchor_spl::token::transfer(context, fee_amount)?;

    custody.collected_fees = math::checked_add(custody.collected_fees, fee_amount)?;

    let oracle_price = OraclePrice::new_from_oracle(
        custody.oracle_type,
        &oracle_account,
        custody.max_oracle_price_error,
        custody.max_oracle_price_age_sec,
        curtime,
    )?;
    let fees_in_usdc = math::to_token_amount(
        oracle_price.get_asset_value_usd(fee_amount, custody.decimals)?,
        6,
    )?;

    launchpad.collected_fees.invalid_bid_usdc = launchpad
        .collected_fees
        .invalid_bid_usdc
        .wrapping_add(fees_in_usdc);

    Ok(())
}

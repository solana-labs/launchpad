use anchor_lang::prelude::*;

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct BidderStats {
    pub fills_volume: u64,
    pub weighted_fills_sum: u128,
    pub min_fill_price: u64,
    pub max_fill_price: u64,
    pub num_trades: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct AuctionStats {
    pub first_trade_time: i64,
    pub last_trade_time: i64,
    pub last_amount: u64,
    pub last_price: u64,
    pub wl_bidders: BidderStats,
    pub reg_bidders: BidderStats,
}

#[derive(Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct CommonParams {
    pub name: String,
    pub description: String,
    pub about_seller: String,
    pub seller_link: String,
    pub start_time: i64,
    pub end_time: i64,
    pub presale_start_time: i64,
    pub presale_end_time: i64,
    pub fill_limit_reg_address: u64,
    pub fill_limit_wl_address: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PaymentParams {
    pub accept_sol: bool,
    pub accept_usdc: bool,
    pub accept_other_tokens: bool,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum PricingModel {
    Fixed,
    DynamicDutchAuction,
}

impl Default for PricingModel {
    fn default() -> Self {
        Self::Fixed
    }
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum RepriceFunction {
    Linear,
}

impl Default for RepriceFunction {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum AmountFunction {
    Fixed,
}

impl Default for AmountFunction {
    fn default() -> Self {
        Self::Fixed
    }
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PricingParams {
    pub custody: Pubkey,
    pub pricing_model: PricingModel,
    pub start_price: u64,
    pub max_price: u64,
    pub min_price: u64,
    pub reprice_delay: i64,
    pub reprice_function: RepriceFunction,
    pub amount_function: AmountFunction,
    pub amount_per_level: u64,
    pub tick_size: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct AuctionToken {
    // Token ratios determine likelihood of getting a particular token if
    // multiple offered. If set to zero, then the available amount will be
    // replace it on the first trade. E.g., if an auction offers 3 tokens,
    // supplied amount of the third token is 3000, and ratios are set to
    // [1230, 2000, 0], then upon the first trade third ratio will be set
    // to 3000, and the buyer will randomly get one of the tokens with
    // 1.23:2:3 probability (i.e. third token is about 2.5 times more
    // likely than first).
    pub ratio: u64,
    pub account: Pubkey,
}

#[account]
#[derive(Default, Debug)]
pub struct Auction {
    pub owner: Pubkey,

    pub enabled: bool,
    pub updatable: bool,
    pub fixed_amount: bool,

    pub common: CommonParams,
    pub payment: PaymentParams,
    pub pricing: PricingParams,
    pub stats: AuctionStats,
    pub tokens: [AuctionToken; 4], // Auction::MAX_TOKENS
    pub num_tokens: u8,

    // time of creation, also used as current wall clock time for testing
    pub creation_time: i64,
    pub update_time: i64,
    pub bump: u8,
}

impl CommonParams {
    pub fn validate(&self, curtime: i64) -> bool {
        (self.fill_limit_reg_address > 0 || self.fill_limit_wl_address > 0)
            && ((self.end_time == 0 && self.start_time == 0)
                || (self.end_time > self.start_time && self.end_time > curtime))
            && ((self.presale_end_time == 0 && self.presale_start_time == 0)
                || (self.presale_end_time > self.presale_start_time
                    && self.presale_end_time > curtime
                    && ((self.end_time == 0 && self.start_time == 0)
                        || self.presale_end_time <= self.start_time)))
    }
}

impl PaymentParams {
    pub fn validate(&self) -> bool {
        self.accept_sol || self.accept_usdc || self.accept_other_tokens
    }
}

impl PricingParams {
    pub fn validate(&self) -> bool {
        ((self.pricing_model == PricingModel::Fixed
            && self.min_price == self.start_price
            && self.max_price == self.start_price)
            || (self.pricing_model != PricingModel::Fixed
                && self.max_price >= self.start_price
                && self.max_price >= self.min_price
                && self.start_price >= self.min_price))
            && self.reprice_delay >= 0
            && (self.pricing_model == PricingModel::Fixed
                || (self.amount_per_level > 0 && self.tick_size > 0))
    }
}

impl Auction {
    pub const LEN: usize = 8 + std::mem::size_of::<Auction>();
    pub const MAX_TOKENS: usize = 4;

    pub fn validate(&self) -> Result<bool> {
        Ok(self.common.name.len() >= 6
            && self.common.validate(self.get_time()?)
            && self.payment.validate()
            && self.pricing.validate())
    }

    /// Checks if the auction is ended
    pub fn is_ended(&self) -> Result<bool> {
        Ok(self.get_time()? >= self.common.end_time)
    }

    #[cfg(feature = "test")]
    pub fn get_time(&self) -> Result<i64> {
        Ok(self.creation_time)
    }

    #[cfg(not(feature = "test"))]
    pub fn get_time(&self) -> Result<i64> {
        let time = solana_program::sysvar::clock::Clock::get()?.unix_timestamp;
        if time > 0 {
            Ok(time)
        } else {
            Err(ProgramError::InvalidAccountData.into())
        }
    }

    pub fn get_auction_amount(&self, price: u64) -> Result<u64> {
        Ok(0)
    }

    pub fn get_auction_price(&self, amount: u64) -> Result<u64> {
        Ok(0)
    }
}

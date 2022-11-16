use {crate::math, anchor_lang::prelude::*};

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
    pub order_limit_reg_address: u64,
    pub order_limit_wl_address: u64,
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
    Exponential,
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
    pub reprice_coef: f64,
    pub reprice_function: RepriceFunction,
    pub amount_function: AmountFunction,
    pub amount_per_level: u64,
    pub tick_size: u64,
    pub unit_size: u64,
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
        self.fill_limit_reg_address >= self.order_limit_reg_address
            && self.fill_limit_wl_address >= self.order_limit_wl_address
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
            && self.unit_size > 0
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

    /// checks if auction has started
    pub fn is_started(&self, curtime: i64, whitelisted: bool) -> bool {
        let auction_start_time = self.get_start_time(whitelisted);
        auction_start_time > 0 && curtime >= auction_start_time
    }

    /// Checks if the auction is ended
    pub fn is_ended(&self, curtime: i64, whitelisted: bool) -> bool {
        curtime >= self.get_end_time(whitelisted)
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

    pub fn get_start_time(&self, whitelisted: bool) -> i64 {
        if whitelisted {
            if self.common.presale_start_time > 0 {
                self.common.presale_start_time
            } else {
                self.common.start_time
            }
        } else {
            self.common.start_time
        }
    }

    pub fn get_end_time(&self, whitelisted: bool) -> i64 {
        if whitelisted {
            std::cmp::max(self.common.presale_end_time, self.common.end_time)
        } else {
            self.common.end_time
        }
    }

    pub fn get_auction_amount(&self, price: u64, curtime: i64) -> Result<u64> {
        match self.pricing.pricing_model {
            PricingModel::Fixed => self.get_auction_amount_fixed(),
            PricingModel::DynamicDutchAuction => self.get_auction_amount_dda(price, curtime),
        }
    }

    pub fn get_auction_price(&self, amount: u64, curtime: i64) -> Result<u64> {
        match self.pricing.pricing_model {
            PricingModel::Fixed => self.get_auction_price_fixed(),
            PricingModel::DynamicDutchAuction => self.get_auction_price_dda(amount, curtime),
        }
    }

    fn get_auction_amount_fixed(&self) -> Result<u64> {
        Ok(u64::MAX)
    }

    fn get_auction_price_fixed(&self) -> Result<u64> {
        Ok(self.pricing.start_price)
    }

    fn get_auction_amount_dda(&self, price: u64, curtime: i64) -> Result<u64> {
        // compute current best offer price
        let best_offer_price = self.get_best_offer_price(curtime)?;

        // return early if user's price is not aggressive enough
        if price < best_offer_price {
            return Ok(0);
        }

        // compute number of price levels
        let price_levels = math::checked_add(
            math::checked_div(
                math::checked_sub(price, best_offer_price)?,
                self.pricing.tick_size,
            )?,
            1,
        )?;

        // compute available amount
        self.get_offer_size(price_levels)
    }

    fn get_auction_price_dda(&self, amount: u64, curtime: i64) -> Result<u64> {
        if amount == 0 {
            return Ok(0);
        }

        // compute current best offer price
        let best_offer_price = self.get_best_offer_price(curtime)?;

        // get number of price levels required to take
        let price_levels = math::checked_sub(
            math::checked_ceil_div(amount, self.pricing.amount_per_level)?,
            1,
        )?;

        // compute the auction price
        let price = math::checked_add(
            best_offer_price,
            math::checked_mul(price_levels, self.pricing.tick_size)?,
        )?;

        Ok(std::cmp::min(price, self.pricing.max_price))
    }

    fn get_best_offer_price(&self, curtime: i64) -> Result<u64> {
        let (last_price, mut last_trade_time) = if self.stats.last_trade_time > 0 {
            (self.stats.last_price, self.stats.last_trade_time)
        } else {
            let start_time = if self.common.start_time > 0 && curtime >= self.common.start_time {
                self.common.start_time
            } else {
                self.get_start_time(true)
            };
            (self.pricing.start_price, start_time)
        };
        last_trade_time = math::checked_add(last_trade_time, self.pricing.reprice_delay)?;
        let end_time = self.get_end_time(true);
        if curtime <= last_trade_time || curtime >= end_time {
            return Ok(last_price);
        }
        let step = math::checked_float_div(
            math::checked_sub(curtime, last_trade_time)? as f64,
            math::checked_sub(end_time, last_trade_time)? as f64,
        )?;

        let mut best_offer_price = match self.pricing.reprice_function {
            RepriceFunction::Exponential => math::checked_as_u64(math::checked_div(
                math::checked_mul(
                    last_price as u128,
                    math::checked_as_u128(math::checked_float_mul(
                        f64::exp(
                            -self.pricing.reprice_coef * math::checked_float_mul(step, 100f64)?,
                        ),
                        10000.0,
                    )?)?,
                )?,
                10000u128,
            )?)?,
            RepriceFunction::Linear => math::checked_as_u64(math::checked_div(
                math::checked_mul(
                    last_price as u128,
                    math::checked_as_u128(math::checked_float_mul(1.0 - step, 10000.0)?)?,
                )?,
                10000u128,
            )?)?,
        };

        // round to tick size
        if best_offer_price % self.pricing.tick_size != 0 {
            best_offer_price = math::checked_mul(
                math::checked_ceil_div(best_offer_price, self.pricing.tick_size)?,
                self.pricing.tick_size,
            )?;
        }

        // check for min/max
        best_offer_price = std::cmp::min(best_offer_price, self.pricing.max_price);
        best_offer_price = std::cmp::max(best_offer_price, self.pricing.min_price);

        Ok(best_offer_price)
    }

    pub fn get_offer_size(&self, price_levels: u64) -> Result<u64> {
        match self.pricing.amount_function {
            AmountFunction::Fixed => math::checked_mul(price_levels, self.pricing.amount_per_level),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_fixture() -> Auction {
        let mut auction = Auction {
            creation_time: 100,
            ..Default::default()
        };

        auction.common.name = "test_auction".to_string();
        auction.common.start_time = 350;
        auction.common.end_time = 500;
        auction.common.presale_start_time = 200;
        auction.common.presale_end_time = 300;

        auction.pricing.pricing_model = PricingModel::DynamicDutchAuction;
        auction.pricing.start_price = 1000;
        auction.pricing.max_price = 2000;
        auction.pricing.min_price = 50;
        auction.pricing.reprice_delay = 10;
        auction.pricing.reprice_coef = 0.05;
        auction.pricing.reprice_function = RepriceFunction::Exponential;
        auction.pricing.amount_function = AmountFunction::Fixed;
        auction.pricing.amount_per_level = 20;
        auction.pricing.tick_size = 10;
        auction.pricing.unit_size = 100;

        auction.payment.accept_sol = true;

        assert!(auction.validate().unwrap());

        auction
    }

    #[test]
    fn get_best_offer_price_exp() {
        let mut auction = get_fixture();

        auction.pricing.reprice_function = RepriceFunction::Exponential;
        assert_eq!(1000, auction.get_best_offer_price(100).unwrap());
        assert_eq!(1000, auction.get_best_offer_price(200).unwrap());
        assert_eq!(510, auction.get_best_offer_price(250).unwrap());
        assert_eq!(1000, auction.get_best_offer_price(350).unwrap());
        assert_eq!(240, auction.get_best_offer_price(400).unwrap());
        assert_eq!(50, auction.get_best_offer_price(499).unwrap());
    }

    #[test]
    fn get_best_offer_price_linear() {
        let mut auction = get_fixture();

        auction.pricing.reprice_function = RepriceFunction::Linear;
        assert_eq!(1000, auction.get_best_offer_price(100).unwrap());
        assert_eq!(1000, auction.get_best_offer_price(200).unwrap());
        assert_eq!(870, auction.get_best_offer_price(250).unwrap());
        assert_eq!(1000, auction.get_best_offer_price(350).unwrap());
        assert_eq!(720, auction.get_best_offer_price(400).unwrap());
        assert_eq!(50, auction.get_best_offer_price(499).unwrap());
    }

    #[test]
    fn get_auction_price_dda() {
        let mut auction = get_fixture();

        auction.pricing.reprice_function = RepriceFunction::Exponential;
        assert_eq!(1000, auction.get_auction_price_dda(1, 100).unwrap());
        assert_eq!(1000, auction.get_auction_price_dda(1, 200).unwrap());
        assert_eq!(510, auction.get_auction_price_dda(1, 250).unwrap());
        assert_eq!(1000, auction.get_auction_price_dda(1, 350).unwrap());
        assert_eq!(240, auction.get_auction_price_dda(1, 400).unwrap());
        assert_eq!(50, auction.get_auction_price_dda(1, 499).unwrap());

        assert_eq!(1000, auction.get_auction_price_dda(20, 100).unwrap());
        assert_eq!(1000, auction.get_auction_price_dda(20, 200).unwrap());
        assert_eq!(510, auction.get_auction_price_dda(20, 250).unwrap());
        assert_eq!(1000, auction.get_auction_price_dda(20, 350).unwrap());
        assert_eq!(240, auction.get_auction_price_dda(20, 400).unwrap());
        assert_eq!(50, auction.get_auction_price_dda(20, 499).unwrap());

        assert_eq!(1010, auction.get_auction_price_dda(21, 100).unwrap());
        assert_eq!(1010, auction.get_auction_price_dda(21, 200).unwrap());
        assert_eq!(520, auction.get_auction_price_dda(21, 250).unwrap());
        assert_eq!(1010, auction.get_auction_price_dda(21, 350).unwrap());
        assert_eq!(250, auction.get_auction_price_dda(21, 400).unwrap());
        assert_eq!(60, auction.get_auction_price_dda(21, 499).unwrap());

        assert_eq!(1090, auction.get_auction_price_dda(200, 100).unwrap());
        assert_eq!(1090, auction.get_auction_price_dda(200, 200).unwrap());
        assert_eq!(600, auction.get_auction_price_dda(200, 250).unwrap());
        assert_eq!(1090, auction.get_auction_price_dda(200, 350).unwrap());
        assert_eq!(330, auction.get_auction_price_dda(200, 400).unwrap());
        assert_eq!(140, auction.get_auction_price_dda(200, 499).unwrap());

        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 100).unwrap());
        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 200).unwrap());
        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 250).unwrap());
        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 350).unwrap());
        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 400).unwrap());
        assert_eq!(2000, auction.get_auction_price_dda(u64::MAX, 499).unwrap());
    }

    #[test]
    fn get_auction_amount_dda() {
        let mut auction = get_fixture();

        auction.pricing.reprice_function = RepriceFunction::Exponential;
        assert_eq!(0, auction.get_auction_amount_dda(0, 100).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(0, 200).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(0, 250).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(0, 350).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(0, 400).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(0, 499).unwrap());

        assert_eq!(0, auction.get_auction_amount_dda(999, 100).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(999, 200).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(509, 250).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(999, 350).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(239, 400).unwrap());
        assert_eq!(0, auction.get_auction_amount_dda(49, 499).unwrap());

        assert_eq!(20, auction.get_auction_amount_dda(1000, 100).unwrap());
        assert_eq!(20, auction.get_auction_amount_dda(1000, 200).unwrap());
        assert_eq!(20, auction.get_auction_amount_dda(510, 250).unwrap());
        assert_eq!(20, auction.get_auction_amount_dda(1000, 350).unwrap());
        assert_eq!(20, auction.get_auction_amount_dda(240, 400).unwrap());
        assert_eq!(20, auction.get_auction_amount_dda(50, 499).unwrap());

        assert_eq!(40, auction.get_auction_amount_dda(1010, 100).unwrap());
        assert_eq!(40, auction.get_auction_amount_dda(1010, 200).unwrap());
        assert_eq!(40, auction.get_auction_amount_dda(520, 250).unwrap());
        assert_eq!(40, auction.get_auction_amount_dda(1010, 350).unwrap());
        assert_eq!(40, auction.get_auction_amount_dda(250, 400).unwrap());
        assert_eq!(40, auction.get_auction_amount_dda(60, 499).unwrap());

        assert_eq!(2020, auction.get_auction_amount_dda(2000, 100).unwrap());
        assert_eq!(2020, auction.get_auction_amount_dda(2000, 200).unwrap());
        assert_eq!(3000, auction.get_auction_amount_dda(2000, 250).unwrap());
        assert_eq!(2020, auction.get_auction_amount_dda(2000, 350).unwrap());
        assert_eq!(3540, auction.get_auction_amount_dda(2000, 400).unwrap());
        assert_eq!(3920, auction.get_auction_amount_dda(2000, 499).unwrap());

        assert_eq!(
            u64::MAX - 15,
            auction
                .get_auction_amount_dda(u64::MAX / 2 + 990, 100)
                .unwrap()
        );
    }
}

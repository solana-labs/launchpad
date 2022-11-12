use anchor_lang::prelude::*;

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum BidType {
    Ioc,
    Fok,
}

impl Default for BidType {
    fn default() -> Self {
        Self::Ioc
    }
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum BadBidType {
    None,
    TooEarly,
    FillLimit,
}

impl Default for BadBidType {
    fn default() -> Self {
        Self::None
    }
}

#[account]
#[derive(Default, Debug)]
pub struct Bid {
    pub owner: Pubkey,
    pub auction: Pubkey,
    pub whitelisted: bool,
    pub seller_initialized: bool,
    pub bid_time: i64,
    pub bid_price: u64,
    pub bid_amount: u64,
    pub bid_type: BidType,
    pub filled: u64,
    pub fill_time: i64,
    pub fill_price: u64,
    pub fill_amount: u64,
    pub bump: u8,
}

impl Bid {
    pub const LEN: usize = 8 + std::mem::size_of::<Bid>();
}

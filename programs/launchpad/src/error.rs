//! Error types

use anchor_lang::prelude::*;

#[error_code]
pub enum LaunchpadError {
    #[msg("Account is not authorized to sign this instruction")]
    MultisigAccountNotAuthorized,
    #[msg("Account has already signed this instruction")]
    MultisigAlreadySigned,
    #[msg("This instruction has already been executed")]
    MultisigAlreadyExecuted,
    #[msg("Invalid launchpad config")]
    InvalidLaunchpadConfig,
    #[msg("Invalid custody config")]
    InvalidCustodyConfig,
    #[msg("Invalid auction config")]
    InvalidAuctionConfig,
    #[msg("Invalid pricing config")]
    InvalidPricingConfig,
    #[msg("Invalid token amount")]
    InvalidTokenAmount,
    #[msg("Too many remaining accounts")]
    TooManyAccountKeys,
    #[msg("Invalid bid account address")]
    InvalidBidAddress,
    #[msg("Invalid dispensing account address")]
    InvalidDispenserAddress,
    #[msg("Dispensing accounts should have the same decimals")]
    InvalidDispenserDecimals,
    #[msg("Invalid seller's balance address")]
    InvalidSellerBalanceAddress,
    #[msg("New auctions are not allowed at this time")]
    NewAuctionsNotAllowed,
    #[msg("Auction updates are not allowed at this time")]
    AuctionUpdatesNotAllowed,
    #[msg("Auction refills are not allowed at this time")]
    AuctionRefillsNotAllowed,
    #[msg("Auction pull-outs are not allowed at this time")]
    AuctionPullOutsNotAllowed,
    #[msg("Bids are not allowed at this time")]
    BidsNotAllowed,
    #[msg("Withdrawals are not allowed at this time")]
    WithdrawalsNotAllowed,
    #[msg("Instruction is not allowed in production")]
    InvalidEnvironment,
    #[msg("Auction hasn't started")]
    AuctionNotStarted,
    #[msg("Auction has been ended")]
    AuctionEnded,
    #[msg("Auction is empty")]
    AuctionEmpty,
    #[msg("Auction is not empty")]
    AuctionNotEmpty,
    #[msg("Auction is not updatable")]
    AuctionNotUpdatable,
    #[msg("Auction with fixed amount")]
    AuctionWithFixedAmount,
    #[msg("Auction is still in progress")]
    AuctionInProgress,
    #[msg("Overflow in arithmetic operation")]
    MathOverflow,
    #[msg("Unsupported price oracle")]
    UnsupportedOracle,
    #[msg("Invalid oracle account")]
    InvalidOracleAccount,
    #[msg("Invalid oracle state")]
    InvalidOracleState,
    #[msg("Stale oracle price")]
    StaleOraclePrice,
    #[msg("Invalid oracle price")]
    InvalidOraclePrice,
    #[msg("Insufficient amount available at the given price")]
    InsufficientAmount,
    #[msg("Bid amount is too large")]
    BidAmountTooLarge,
    #[msg("Bid price is too small")]
    BidPriceTooSmall,
    #[msg("Fill limit exceeded")]
    FillAmountLimit,
    #[msg("Unexpected price calculation error")]
    PriceCalcError,
    #[msg("This instruction must be all alone in the transaction")]
    MustBeSingleInstruction,
}

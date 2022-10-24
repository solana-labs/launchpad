//! Launchpad program entrypoint

mod error;
mod instructions;
mod math;
mod oracle;
mod state;

use {anchor_lang::prelude::*, instructions::*};

solana_security_txt::security_txt! {
    name: "Launchpad",
    project_url: "https://github.com/solana-labs/solana-program-library/tree/master/launchpad",
    contacts: "email:solana.farms@protonmail.com",
    policy: "",
    preferred_languages: "en",
    auditors: ""
}

declare_id!("LPD1BCWvd499Rk7aG5zG8uieUTTqba1JaYkUpXjUN9q");

#[derive(Clone)]
pub struct Launchpad;

impl anchor_lang::Id for Launchpad {
    fn id() -> Pubkey {
        ID
    }
}

#[program]
pub mod launchpad {
    use super::*;

    pub fn init(ctx: Context<Init>, params: InitParams) -> Result<()> {
        instructions::init(ctx, &params)
    }

    pub fn set_admin_signers<'info>(
        ctx: Context<'_, '_, '_, 'info, SetAdminSigners<'info>>,
        params: SetAdminSignersParams,
    ) -> Result<u8> {
        instructions::set_admin_signers(ctx, &params)
    }

    pub fn set_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, SetFees<'info>>,
        params: SetFeesParams,
    ) -> Result<u8> {
        instructions::set_fees(ctx, &params)
    }

    pub fn set_oracle_config<'info>(
        ctx: Context<'_, '_, '_, 'info, SetOracleConfig<'info>>,
        params: SetOracleConfigParams,
    ) -> Result<u8> {
        instructions::set_oracle_config(ctx, &params)
    }

    pub fn set_permissions<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPermissions<'info>>,
        params: SetPermissionsParams,
    ) -> Result<u8> {
        instructions::set_permissions(ctx, &params)
    }

    pub fn withdraw_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawFees<'info>>,
        params: WithdrawFeesParams,
    ) -> Result<u8> {
        instructions::withdraw_fees(ctx, &params)
    }

    pub fn test_init(ctx: Context<TestInit>, params: TestInitParams) -> Result<()> {
        instructions::test_init(ctx, &params)
    }

    pub fn delete_auction<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteAuction<'info>>,
        params: DeleteAuctionParams,
    ) -> Result<u8> {
        instructions::delete_auction(ctx, &params)
    }

    pub fn set_test_oracle_price<'info>(
        ctx: Context<'_, '_, '_, 'info, SetTestOraclePrice<'info>>,
        params: SetTestOraclePriceParams,
    ) -> Result<u8> {
        instructions::set_test_oracle_price(ctx, &params)
    }

    pub fn set_test_time<'info>(
        ctx: Context<'_, '_, '_, 'info, SetTestTime<'info>>,
        params: SetTestTimeParams,
    ) -> Result<u8> {
        instructions::set_test_time(ctx, &params)
    }

    pub fn add_tokens(ctx: Context<AddTokens>, params: AddTokensParams) -> Result<u8> {
        instructions::add_tokens(ctx, &params)
    }

    pub fn init_auction(ctx: Context<InitAuction>, params: InitAuctionParams) -> Result<u8> {
        instructions::init_auction(ctx, &params)
    }

    pub fn init_token(ctx: Context<InitToken>, params: InitTokenParams) -> Result<u8> {
        instructions::init_token(ctx, &params)
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, params: MintTokensParams) -> Result<u8> {
        instructions::mint_tokens(ctx, &params)
    }

    pub fn remove_tokens(ctx: Context<RemoveTokens>, params: RemoveTokensParams) -> Result<u8> {
        instructions::remove_tokens(ctx, &params)
    }

    pub fn enable_auction(ctx: Context<EnableAuction>, params: EnableAuctionParams) -> Result<u8> {
        instructions::enable_auction(ctx, &params)
    }

    pub fn disable_auction(
        ctx: Context<DisableAuction>,
        params: DisableAuctionParams,
    ) -> Result<u8> {
        instructions::disable_auction(ctx, &params)
    }

    pub fn update_auction(ctx: Context<UpdateAuction>, params: UpdateAuctionParams) -> Result<u8> {
        instructions::update_auction(ctx, &params)
    }

    pub fn whitelist_add(ctx: Context<WhitelistAdd>, params: WhitelistAddParams) -> Result<u8> {
        instructions::whitelist_add(ctx, &params)
    }

    pub fn whitelist_remove(
        ctx: Context<WhitelistRemove>,
        params: WhitelistRemoveParams,
    ) -> Result<u8> {
        instructions::whitelist_remove(ctx, &params)
    }

    pub fn withdraw_funds(ctx: Context<WithdrawFunds>, params: WithdrawFundsParams) -> Result<u8> {
        instructions::withdraw_funds(ctx, &params)
    }

    pub fn place_bid(ctx: Context<PlaceBid>, params: PlaceBidParams) -> Result<u8> {
        instructions::place_bid(ctx, &params)
    }

    pub fn cancel_bid(ctx: Context<CancelBid>, params: CancelBidParams) -> Result<u8> {
        instructions::cancel_bid(ctx, &params)
    }

    pub fn get_auction_price(
        ctx: Context<GetAuctionPrice<'info>>,
        params: GetAuctionPriceParams,
    ) -> Result<u8> {
        instructions::get_auction_price(ctx, &params)
    }

    pub fn get_auction_amount<'info>(
        ctx: Context<'_, '_, '_, 'info, GetAuctionAmount<'info>>,
        params: GetAuctionAmountParams,
    ) -> Result<u8> {
        instructions::get_auction_amount(ctx, &params)
    }
}

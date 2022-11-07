//! Launchpad program entrypoint

mod error;
mod instructions;
mod math;
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

#[program]
pub mod launchpad {
    use super::*;

    // admin instructions

    pub fn delete_auction<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteAuction<'info>>,
        params: DeleteAuctionParams,
    ) -> Result<u8> {
        instructions::delete_auction(ctx, &params)
    }

    pub fn init(ctx: Context<Init>, params: InitParams) -> Result<()> {
        instructions::init(ctx, &params)
    }

    pub fn init_custody<'info>(
        ctx: Context<'_, '_, '_, 'info, InitCustody<'info>>,
        params: InitCustodyParams,
    ) -> Result<u8> {
        instructions::init_custody(ctx, &params)
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

    // test instructions

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

    pub fn test_init(ctx: Context<TestInit>, params: TestInitParams) -> Result<()> {
        instructions::test_init(ctx, &params)
    }

    // seller instructions

    pub fn add_tokens(ctx: Context<AddTokens>, params: AddTokensParams) -> Result<()> {
        instructions::add_tokens(ctx, &params)
    }

    pub fn disable_auction(
        ctx: Context<DisableAuction>,
        params: DisableAuctionParams,
    ) -> Result<()> {
        instructions::disable_auction(ctx, &params)
    }

    pub fn enable_auction(ctx: Context<EnableAuction>, params: EnableAuctionParams) -> Result<()> {
        instructions::enable_auction(ctx, &params)
    }

    pub fn init_auction<'info>(
        ctx: Context<'_, '_, '_, 'info, InitAuction<'info>>,
        params: InitAuctionParams,
    ) -> Result<()> {
        instructions::init_auction(ctx, &params)
    }

    pub fn remove_tokens(ctx: Context<RemoveTokens>, params: RemoveTokensParams) -> Result<()> {
        instructions::remove_tokens(ctx, &params)
    }

    pub fn update_auction(ctx: Context<UpdateAuction>, params: UpdateAuctionParams) -> Result<()> {
        instructions::update_auction(ctx, &params)
    }

    pub fn whitelist_add<'info>(
        ctx: Context<'_, '_, '_, 'info, WhitelistAdd<'info>>,
        params: WhitelistAddParams,
    ) -> Result<()> {
        instructions::whitelist_add(ctx, &params)
    }

    pub fn whitelist_remove<'info>(
        ctx: Context<'_, '_, '_, 'info, WhitelistRemove<'info>>,
        params: WhitelistRemoveParams,
    ) -> Result<()> {
        instructions::whitelist_remove(ctx, &params)
    }

    pub fn withdraw_funds(ctx: Context<WithdrawFunds>, params: WithdrawFundsParams) -> Result<()> {
        instructions::withdraw_funds(ctx, &params)
    }

    // buyer instructions

    pub fn cancel_bid(ctx: Context<CancelBid>, params: CancelBidParams) -> Result<()> {
        instructions::cancel_bid(ctx, &params)
    }

    pub fn get_auction_amount(
        ctx: Context<GetAuctionAmount>,
        params: GetAuctionAmountParams,
    ) -> Result<u64> {
        instructions::get_auction_amount(ctx, &params)
    }

    pub fn get_auction_price(
        ctx: Context<GetAuctionPrice>,
        params: GetAuctionPriceParams,
    ) -> Result<u64> {
        instructions::get_auction_price(ctx, &params)
    }

    pub fn place_bid<'info>(
        ctx: Context<'_, '_, '_, 'info, PlaceBid<'info>>,
        params: PlaceBidParams,
    ) -> Result<()> {
        instructions::place_bid(ctx, &params)
    }
}

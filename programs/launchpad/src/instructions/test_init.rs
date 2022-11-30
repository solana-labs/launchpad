//! TestInit instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{
            launchpad::{Fee, Launchpad},
            multisig::Multisig,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
    solana_address_lookup_table_program as saltp,
    solana_program::{program, program_error::ProgramError, sysvar},
};

#[derive(Accounts)]
pub struct TestInit<'info> {
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    #[account(
        init,
        payer = upgrade_authority,
        space = Multisig::LEN,
        seeds = [b"multisig"],
        bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(
        init,
        payer = upgrade_authority,
        space = 0,
        seeds = [b"transfer_authority"],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = upgrade_authority,
        space = Launchpad::LEN,
        seeds = [b"launchpad"],
        bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,

    /// CHECK: lookup table account
    #[account(mut)]
    pub lookup_table: AccountInfo<'info>,

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

    /// CHECK: lookup table program
    lookup_table_program: AccountInfo<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    // remaining accounts: 1 to Multisig::MAX_SIGNERS admin signers (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TestInitParams {
    pub min_signatures: u8,
    pub allow_new_auctions: bool,
    pub allow_auction_updates: bool,
    pub allow_auction_refills: bool,
    pub allow_auction_pullouts: bool,
    pub allow_new_bids: bool,
    pub allow_withdrawals: bool,
    pub new_auction_fee: u64,
    pub auction_update_fee: u64,
    pub invalid_bid_fee: Fee,
    pub trade_fee: Fee,
    pub recent_slot: u64,
}

pub fn test_init(ctx: Context<TestInit>, params: &TestInitParams) -> Result<()> {
    if !cfg!(feature = "test") {
        return err!(LaunchpadError::InvalidEnvironment);
    }

    // initialize multisig, this will fail if account is already initialized
    let mut multisig = ctx.accounts.multisig.load_init()?;

    multisig.set_signers(ctx.remaining_accounts, params.min_signatures)?;

    // record multisig PDA bump
    multisig.bump = *ctx
        .bumps
        .get("multisig")
        .ok_or(ProgramError::InvalidSeeds)?;

    // record launchpad
    let launchpad = ctx.accounts.launchpad.as_mut();
    launchpad.permissions.allow_new_auctions = params.allow_new_auctions;
    launchpad.permissions.allow_auction_updates = params.allow_auction_updates;
    launchpad.permissions.allow_auction_refills = params.allow_auction_refills;
    launchpad.permissions.allow_auction_pullouts = params.allow_auction_pullouts;
    launchpad.permissions.allow_new_bids = params.allow_new_bids;
    launchpad.permissions.allow_withdrawals = params.allow_withdrawals;
    launchpad.fees.new_auction = params.new_auction_fee;
    launchpad.fees.auction_update = params.auction_update_fee;
    launchpad.fees.invalid_bid = params.invalid_bid_fee;
    launchpad.fees.trade = params.trade_fee;
    launchpad.collected_fees.new_auction_sol = 0;
    launchpad.collected_fees.auction_update_sol = 0;
    launchpad.collected_fees.invalid_bid_usdc = 0;
    launchpad.collected_fees.trade_usdc = 0;
    launchpad.transfer_authority_bump = *ctx
        .bumps
        .get("transfer_authority")
        .ok_or(ProgramError::InvalidSeeds)?;
    launchpad.launchpad_bump = *ctx
        .bumps
        .get("launchpad")
        .ok_or(ProgramError::InvalidSeeds)?;

    if !launchpad.validate() {
        return err!(LaunchpadError::InvalidLaunchpadConfig);
    }

    // initialize lookup-table
    let transfer_authority = ctx.accounts.transfer_authority.key();
    let payer = ctx.accounts.upgrade_authority.key();
    let (init_table_ix, table_address) =
        saltp::instruction::create_lookup_table(transfer_authority, payer, params.recent_slot);
    require_keys_eq!(table_address, ctx.accounts.lookup_table.key());
    require_keys_eq!(ctx.accounts.lookup_table_program.key(), saltp::ID);

    let authority_seeds: &[&[&[u8]]] =
        &[&[b"transfer_authority", &[launchpad.transfer_authority_bump]]];
    program::invoke_signed(
        &init_table_ix,
        &[
            ctx.accounts.lookup_table.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.upgrade_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        authority_seeds,
    )?;

    // add addresses to the lookup table
    let extend_table_ix = saltp::instruction::extend_lookup_table(
        table_address,
        transfer_authority,
        Some(payer),
        vec![
            transfer_authority,
            ctx.accounts.launchpad.key(),
            ctx.accounts.recent_slothashes.key(),
            ctx.accounts.instructions.key(),
            ctx.accounts.system_program.key(),
            ctx.accounts.token_program.key(),
        ],
    );
    program::invoke_signed(
        &extend_table_ix,
        &[
            ctx.accounts.lookup_table.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.upgrade_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        authority_seeds,
    )?;

    Ok(())
}

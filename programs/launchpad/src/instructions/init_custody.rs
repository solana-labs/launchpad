//! InitCustody instruction handler

use {
    crate::{
        error::LaunchpadError,
        oracle::OracleType,
        state::multisig::{AdminInstruction, Multisig},
    },
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{Mint, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct InitCustody<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut, seeds = [b"multisig"], bump = multisig.load()?.bump)]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(seeds = [b"transfer_authority"], bump)]
    pub transfer_authority: AccountInfo<'info>,

    // instruction can be called multiple times due to multisig use, hence init_if_needed
    // instead of init. On the first call account is zero initialized and filled out when
    // all signatures are collected. When account is in zeroed state it can't be used in other
    // instructions because seeds are computed with recorded mints. Uniqueness is enforced
    // manually in the instruction handler.
    #[account(init_if_needed,
              payer = admin,
              space = Custody::LEN,
              seeds = [b"custody", custody_token_mint.key().as_ref()],
              bump)]
    pub custody: Box<Account<'info, Custody>>,

    pub custody_token_mint: Box<Account<'info, Mint>>,

    // token custodies are shared between multiple auctions
    #[account(init_if_needed,
              payer = admin,
              constraint = custody_token_mint.key() == custody_token_account.mint,
              associated_token::mint = custody_token_mint,
              associated_token::authority = transfer_authority)]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitCustodyParams {
    pub max_oracle_price_error: f64,
    pub max_oracle_price_age_sec: u32,
    pub oracle_type: OracleType,
    pub oracle_account: Pubkey,
}

pub fn init_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, InitCustody<'info>>,
    params: &InitCustodyParams,
) -> Result<u8> {
    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::InitCustody, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // record custody data
    let custody = ctx.accounts.custody.as_mut();
    if custody.mint != Pubkey::default() {
        // return error if custody is already initialized
        return Err(ProgramError::AccountAlreadyInitialized.into());
    }

    custody.token_account = ctx.accounts.custody_token_account.key();
    custody.collected_fees = 0;
    custdoy.mint = ctx.accounts.custody_token_mint.key();
    custody.decimals = ctx.accounts.custody_token_mint.decimals;
    custody.max_oracle_price_error = params.max_oracle_price_error;
    custody.max_oracle_price_age_sec = params.max_oracle_price_age_sec;
    custody.oracle_type = params.oracle_type;
    custody.oracle_account = params.oracle_account;
    custody.bump = *ctx.bumps.get("custody").ok_or(ProgramError::InvalidSeeds)?;

    if !custody.validate() {
        err!(LaunchpadError::InvalidCustodyConfig)
    } else {
        Ok(0)
    }
}

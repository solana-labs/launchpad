//! WithdrawFees instruction handler

use {
    crate::{
        error::LaunchpadError,
        math,
        state::{
            custody::Custody,
            launchpad::Launchpad,
            multisig::{AdminInstruction, Multisig},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account()]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"],
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"], 
        bump = launchpad.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        seeds = [b"launchpad"],
        bump = launchpad.launchpad_bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut,
        seeds = [b"custody", custody.mint.key().as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    #[account(
        mut,
        constraint = custody_token_account.key() == custody.token_account.key()
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawFeesParams {
    pub amount: u64,
}

pub fn withdraw_fees<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawFees<'info>>,
    params: &WithdrawFeesParams,
) -> Result<u8> {
    // validate inputs
    require_gt!(params.amount, 0u64, LaunchpadError::InvalidTokenAmount);

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::WithdrawFees, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // transfer fees from the custody to the receiver
    let custody = ctx.accounts.custody.as_mut();

    if custody.collected_fees < params.amount {
        return Err(ProgramError::InsufficientFunds.into());
    }
    custody.collected_fees = math::checked_sub(custody.collected_fees, params.amount)?;

    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(0)
}

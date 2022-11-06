//! WithdrawFunds instruction handler

use {
    crate::{
        error::LaunchpadError,
        math,
        state::{custody::Custody, launchpad::Launchpad, seller_balance::SellerBalance},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

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
        seeds = [b"custody", custody.mint.as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    #[account(
        mut,
        constraint = custody_token_account.key() == custody.token_account.key()
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        has_one = owner,
        constraint = seller_balance.custody == custody.key(),
        seeds = [b"seller_balance", seller_balance.custody.as_ref()],
        bump = seller_balance.bump
    )]
    pub seller_balance: Box<Account<'info, SellerBalance>>,

    #[account(
        mut,
        constraint = receiving_account.mint == custody_token_account.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawFundsParams {
    pub amount: u64,
}

pub fn withdraw_funds(ctx: Context<WithdrawFunds>, params: &WithdrawFundsParams) -> Result<()> {
    require!(
        ctx.accounts.launchpad.permissions.allow_withdrawals,
        LaunchpadError::WithdrawalsNotAllowed
    );

    // validate inputs
    require_gt!(params.amount, 0u64, LaunchpadError::InvalidTokenAmount);

    // transfer fees from the custody to the receiver
    let seller_balance = ctx.accounts.seller_balance.as_mut();

    if seller_balance.balance < params.amount {
        return Err(ProgramError::InsufficientFunds.into());
    }
    seller_balance.balance = math::checked_sub(seller_balance.balance, params.amount)?;

    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(())
}

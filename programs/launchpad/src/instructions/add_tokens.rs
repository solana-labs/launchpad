//! AddTokens instruction handler

use {
    crate::{error::LaunchpadError, state::{auction::Auction, launchpad::Launchpad}},
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount}
};

#[derive(Accounts)]
pub struct AddTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        constraint = funding_account.mint == dispensing_custody.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        mut, seeds = [b"transfer_authority"], 
        bump = launchpad.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(mut, seeds = [b"launchpad"], bump = launchpad.launchpad_bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    #[account(
        mut,
        seeds = [b"dispense", dispensing_custody.mint.as_ref(), auction.key().as_ref()],
        bump
    )]
    pub dispensing_custody: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddTokensParams {
    pub amount: u64
}

pub fn add_tokens(
    ctx: Context<AddTokens>,
    params: &AddTokensParams,
) -> Result<()> {
    // TODO check dispensing custody is in auction records
    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.dispensing_custody.to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(())
}

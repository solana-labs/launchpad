//! RemoveTokens instruction handler

use {
    crate::{error::LaunchpadError},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct RemoveTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        constraint = receiving_account.mint == dispensing_custody.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    #[account(mut, seeds = [b"launchpad"], bump = launchpad.bump)]
    pub launchpad: Box<Account<'info, Launchpad>>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.name.as_bytes()],
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
pub struct RemoveTokensParams {
    pub amount: u64
}

pub fn remove_tokens(
    ctx: Context<RemoveTokens>,
    params: &RemoveTokensParams,
) -> Result {
    // TODO check dispensing custody is in auction records
    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.dispensing_custody.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(())
}

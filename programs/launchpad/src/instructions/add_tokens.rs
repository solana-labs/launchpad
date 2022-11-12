//! AddTokens instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{auction::Auction, launchpad::Launchpad},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
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
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    pub dispensing_custody_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = owner,
        constraint = dispensing_custody_mint.key() == dispensing_custody.mint,
        token::mint = dispensing_custody_mint,
        token::authority = transfer_authority,
        seeds = [b"dispense", dispensing_custody_mint.key().as_ref(), auction.key().as_ref()],
        bump
    )]
    pub dispensing_custody: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddTokensParams {
    pub amount: u64,
}

pub fn add_tokens(ctx: Context<AddTokens>, params: &AddTokensParams) -> Result<()> {
    if ctx
        .accounts
        .auction
        .is_started(ctx.accounts.auction.get_time()?, true)
    {
        require!(
            ctx.accounts.launchpad.permissions.allow_auction_refills,
            LaunchpadError::AuctionRefillsNotAllowed
        );
    }
    require!(
        !ctx.accounts.auction.fixed_amount,
        LaunchpadError::AuctionWithFixedAmount
    );

    ctx.accounts.launchpad.transfer_tokens(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.dispensing_custody.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(())
}

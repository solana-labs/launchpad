//! WhitelistAdd instruction handler

use {
    crate::{error::LaunchpadError},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct WhitelistAdd<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut, 
        has_one = owner,
        seeds = [b"auction", auction.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    system_program: Program<'info, System>,

    // remaining accounts:
    //   Bid accounts for addresses to be whitelisted (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistAddParams {
    addresses: Vec<Pubkey>
}

pub fn whitelist_add(
    ctx: Context<WhitelistAdd>,
    params: &WhitelistAddParams,
) -> Result {
    if params.addresses.is_empty() || ctx.remaining_accounts.len() != params.addresses.len() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    let bid_accounts = load_accounts::<Bid>(ctx.remaining_accounts)?;
    for (accoubidnt, owner) in bid_accounts.iter().zip(params.addresses) {
        // TODO validate address
        if bid.bump == 0 {
            bid.owner = owner;
            bid.auction = ctx.accounts.auction.key();
            bid.seller_initialized = true;
        }
        bid.whitelisted = true;
    }
    save_accounts(&bid_accounts)?;

    Ok(())
}

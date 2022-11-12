//! WhitelistRemove instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{self, auction::Auction, bid::Bid},
    },
    anchor_lang::{prelude::*, AccountsClose},
};

#[derive(Accounts)]
pub struct WhitelistRemove<'info> {
    #[account()]
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,
    // remaining accounts:
    //   Bid accounts to be removed from the whitelist (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistRemoveParams {}

pub fn whitelist_remove<'info>(
    ctx: Context<'_, '_, '_, 'info, WhitelistRemove<'info>>,
    _params: &WhitelistRemoveParams,
) -> Result<()> {
    if ctx.remaining_accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    let auction_ended = ctx
        .accounts
        .auction
        .is_ended(ctx.accounts.auction.get_time()?, true);
    let mut bid_accounts = state::load_accounts::<Bid>(ctx.remaining_accounts, &crate::ID)?;
    for bid in bid_accounts.iter_mut() {
        // validate bid address
        let expected_bid_key = Pubkey::create_program_address(
            &[
                b"bid",
                bid.owner.as_ref(),
                ctx.accounts.auction.key().as_ref(),
                &[bid.bump],
            ],
            &crate::ID,
        )
        .map_err(|_| LaunchpadError::InvalidBidAddress)?;
        require_keys_eq!(
            bid.key(),
            expected_bid_key,
            LaunchpadError::InvalidBidAddress
        );

        // remove from white-list or close the account
        if auction_ended && bid.seller_initialized {
            bid.close(ctx.accounts.owner.to_account_info())?;
        } else {
            bid.whitelisted = false;
            bid.exit(&crate::ID)?;
        }
    }

    Ok(())
}

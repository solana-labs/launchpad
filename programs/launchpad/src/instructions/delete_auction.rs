//! DeleteAuction instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{
            self,
            auction::Auction,
            multisig::{AdminInstruction, Multisig},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct DeleteAuction<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut, seeds = [b"multisig"], bump = multisig.load()?.bump)]
    pub multisig: AccountLoader<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump,
        close = admin
    )]
    pub auction: Box<Account<'info, Auction>>,

    token_program: Program<'info, Token>,
    // remaining accounts:
    //   1 to Auction::MAX_TOKENS dispensing custody addresses (write, unsigned)
    //      with seeds = [b"dispense", mint.key().as_ref(), auction.key().as_ref()],
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DeleteAuctionParams {}

pub fn delete_auction<'info>(
    ctx: Context<'_, '_, '_, 'info, DeleteAuction<'info>>,
    params: &DeleteAuctionParams,
) -> Result<u8> {
    if !cfg!(feature = "test") {
        return err!(LaunchpadError::InvalidEnvironment);
    }

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::DeleteAuction, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    if !ctx.remaining_accounts.is_empty() {
        let dispensers = state::load_accounts::<TokenAccount>(
            ctx.remaining_accounts,
            &Token::id(),
            Auction::MAX_TOKENS,
        )?;
        // TODO check addresses
        for dispenser in &dispensers {
            if dispenser.owner != crate::ID {
                return Err(ProgramError::IllegalOwner.into());
            }
        }
        for account in &dispensers {
            if account.amount > 0 {
                msg!("Non-empty dispensing account: {}", account.key());
                return err!(LaunchpadError::AuctionNotEmpty);
            }
            // TODO close token account here
        }
    }

    // TODO delete auction

    Ok(0)
}
//! SetPermissions instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{
            launchpad::Launchpad,
            multisig::{AdminInstruction, Multisig},
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetPermissions<'info> {
    #[account()]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"],
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"launchpad"],
        bump = launchpad.launchpad_bump
    )]
    pub launchpad: Box<Account<'info, Launchpad>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetPermissionsParams {
    pub allow_new_auctions: bool,
    pub allow_auction_updates: bool,
    pub allow_auction_refills: bool,
    pub allow_auction_pullouts: bool,
    pub allow_new_bids: bool,
    pub allow_withdrawals: bool,
}

pub fn set_permissions<'info>(
    ctx: Context<'_, '_, '_, 'info, SetPermissions<'info>>,
    params: &SetPermissionsParams,
) -> Result<u8> {
    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::SetPermissions, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // update permissions
    let launchpad = ctx.accounts.launchpad.as_mut();
    launchpad.permissions.allow_new_auctions = params.allow_new_auctions;
    launchpad.permissions.allow_auction_updates = params.allow_auction_updates;
    launchpad.permissions.allow_auction_refills = params.allow_auction_refills;
    launchpad.permissions.allow_auction_pullouts = params.allow_auction_pullouts;
    launchpad.permissions.allow_new_bids = params.allow_new_bids;
    launchpad.permissions.allow_withdrawals = params.allow_withdrawals;

    if !launchpad.validate() {
        err!(LaunchpadError::InvalidLaunchpadConfig)
    } else {
        Ok(0)
    }
}

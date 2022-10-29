pub mod auction;
pub mod bid;
pub mod custody;
pub mod launchpad;
pub mod multisig;
pub mod seller_balance;

use {
    crate::{error::LaunchpadError, math},
    anchor_lang::prelude::*,
    anchor_spl::token::TokenAccount,
};

pub fn is_empty_account(account_info: &AccountInfo) -> Result<bool> {
    Ok(account_info.try_data_is_empty()? || account_info.try_lamports()? == 0)
}

pub fn initialize_account<'info>(
    payer: AccountInfo<'info>,
    target_account: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    owner: &Pubkey,
    seeds: &[&[&[u8]]],
    len: usize,
) -> Result<()> {
    let current_lamports = target_account.try_lamports()?;
    if current_lamports == 0 {
        // if account doesn't have any lamports initialize it with conventional create_account
        let lamports = Rent::get()?.minimum_balance(len);
        let cpi_accounts = anchor_lang::system_program::CreateAccount {
            from: payer,
            to: target_account,
        };
        let cpi_context = anchor_lang::context::CpiContext::new(system_program, cpi_accounts);
        anchor_lang::system_program::create_account(
            cpi_context.with_signer(seeds),
            lamports,
            math::checked_as_u64(len)?,
            owner,
        )?;
    } else {
        // fund the account for rent exemption
        let required_lamports = Rent::get()?
            .minimum_balance(len)
            .saturating_sub(current_lamports);
        if required_lamports > 0 {
            let cpi_accounts = anchor_lang::system_program::Transfer {
                from: payer,
                to: target_account.clone(),
            };
            let cpi_context =
                anchor_lang::context::CpiContext::new(system_program.clone(), cpi_accounts);
            anchor_lang::system_program::transfer(cpi_context, required_lamports)?;
        }
        // allocate space
        let cpi_accounts = anchor_lang::system_program::Allocate {
            account_to_allocate: target_account.clone(),
        };
        let cpi_context =
            anchor_lang::context::CpiContext::new(system_program.clone(), cpi_accounts);
        anchor_lang::system_program::allocate(
            cpi_context.with_signer(seeds),
            math::checked_as_u64(len)?,
        )?;
        // assign to the program
        let cpi_accounts = anchor_lang::system_program::Assign {
            account_to_assign: target_account,
        };
        let cpi_context = anchor_lang::context::CpiContext::new(system_program, cpi_accounts);
        anchor_lang::system_program::assign(cpi_context.with_signer(seeds), owner)?;
    }
    Ok(())
}

pub fn load_accounts<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone>(
    accounts: &[AccountInfo<'a>],
    expected_owner: &Pubkey,
    max_accounts: usize,
) -> Result<Vec<Account<'a, T>>> {
    let mut res: Vec<Account<T>> = Vec::with_capacity(accounts.len());

    for account in accounts {
        if account.owner != expected_owner {
            return Err(ProgramError::IllegalOwner.into());
        }
        res.push(Account::<T>::try_from(account)?);
    }

    if res.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }
    if res.len() > max_accounts {
        return err!(LaunchpadError::TooManyAccountKeys);
    }

    Ok(res)
}

pub fn save_accounts<T: AccountSerialize + AccountDeserialize + Owner + Clone>(
    accounts: &[Account<T>],
) -> Result<()> {
    for account in accounts {
        account.exit(&crate::ID)?;
    }
    Ok(())
}

pub fn create_accounts<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone>(
    accounts: &[AccountInfo<'a>],
    owner: &Pubkey,
    max_accounts: usize,
) -> Result<Vec<Account<'a, T>>> {
    if accounts.len() > max_accounts {
        return err!(LaunchpadError::TooManyAccountKeys);
    }

    // TODO
    let mut res: Vec<Account<T>> = Vec::with_capacity(accounts.len());

    Ok(res)
}

pub fn create_token_accounts<'a>(
    accounts: &[AccountInfo<'a>],
    owner: &Pubkey,
    max_accounts: usize,
) -> Result<Vec<Account<'a, TokenAccount>>> {
    if accounts.len() > max_accounts {
        return err!(LaunchpadError::TooManyAccountKeys);
    }

    // TODO
    let mut res: Vec<Account<TokenAccount>> = Vec::with_capacity(accounts.len());

    Ok(res)
}

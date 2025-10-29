use crate::error::PumpError;
use anchor_lang::prelude::*;
use core::convert::TryFrom;

pub fn calculate_tokens_out(
    sol_amount: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    let sol_amount_u128 = sol_amount as u128;
    let initial_sol_u128 = initial_sol_reserves as u128;
    let initial_token_u128 = initial_token_reserves as u128;

    if initial_sol_u128 == 0 || initial_token_u128 == 0 {
        return err!(PumpError::InvalidConstant);
    }

    let k = initial_sol_u128
        .checked_mul(initial_token_u128)
        .ok_or(PumpError::NumericOverflow)?;

    let new_sol_reserves = initial_sol_u128
        .checked_add(sol_amount_u128)
        .ok_or(PumpError::NumericOverflow)?;

    if new_sol_reserves == 0 {
        return err!(PumpError::DivideByZero);
    }

    let new_token_reserves = k
        .checked_div(new_sol_reserves)
        .ok_or(PumpError::InvalidConstant)?;

    if new_token_reserves > initial_token_u128 {
        return err!(PumpError::InvalidConstant);
    }

    let tokens_out_u128 = initial_token_u128
        .checked_sub(new_token_reserves)
        .ok_or(PumpError::InvalidConstant)?;

    let tokens_out_u64 = u64::try_from(tokens_out_u128).map_err(|_| PumpError::NumericOverflow)?;

    Ok(tokens_out_u64)
}

pub fn calculate_sol_out(
    tokens_in: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    msg!("calculate_sol_out - tokens_in: {}", tokens_in);
    msg!(
        "calculate_sol_out - initial_sol_reserves: {}",
        initial_sol_reserves
    );
    msg!(
        "calculate_sol_out - initial_token_reserves: {}",
        initial_token_reserves
    );

    let tokens_in_u128 = tokens_in as u128;
    let initial_sol_u128 = initial_sol_reserves as u128;
    let initial_token_u128 = initial_token_reserves as u128;

    if initial_sol_u128 == 0 || initial_token_u128 == 0 {
        return err!(PumpError::InvalidConstant);
    }

    let k = initial_sol_u128
        .checked_mul(initial_token_u128)
        .ok_or(PumpError::NumericOverflow)?;

    let new_token_reserves = initial_token_u128
        .checked_add(tokens_in_u128)
        .ok_or(PumpError::NumericOverflow)?;

    let new_sol_reserves = k
        .checked_div(new_token_reserves)
        .ok_or(PumpError::InvalidConstant)?;

    let sol_out_u128 = initial_sol_u128
        .checked_sub(new_sol_reserves)
        .ok_or(PumpError::InvalidConstant)?;

    msg!("calculate_sol_out - k: {}", k);
    msg!(
        "calculate_sol_out - new_token_reserves: {}",
        new_token_reserves
    );
    msg!("calculate_sol_out - new_sol_reserves: {}", new_sol_reserves);
    msg!("calculate_sol_out - sol_out_u128: {}", sol_out_u128);

    let sol_out_u64 = u64::try_from(sol_out_u128).map_err(|_| PumpError::NumericOverflow)?;

    Ok(sol_out_u64)
}

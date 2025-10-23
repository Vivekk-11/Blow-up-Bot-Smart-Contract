use anchor_lang::error::ErrorCode;
use anchor_lang::prelude::*;

pub fn calculate_tokens_out(
    sol_amount: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    let k = initial_sol_reserves
        .checked_mul(initial_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_sol_reserves = initial_sol_reserves
        .checked_add(sol_amount)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_token_reserves = k
        .checked_div(new_sol_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let tokens_out = initial_token_reserves
        .checked_sub(new_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(tokens_out)
}

pub fn calculate_sol_out(
    tokens_in: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    let k = initial_sol_reserves
        .checked_mul(initial_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_token_reserves = initial_token_reserves
        .checked_add(tokens_in)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_sol_reserves = k
        .checked_div(new_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let sol_out = initial_sol_reserves
        .checked_sub(new_sol_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(sol_out)
}

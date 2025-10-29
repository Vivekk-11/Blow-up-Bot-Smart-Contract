use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{
    account::sell_tokens::SellTokens, error::PumpError, math::calculate_sol_out,
    state::bonding_curve::GraduationState,
};

pub fn handler(ctx: Context<SellTokens>, tokens_in: u64) -> Result<()> {
    require!(
        ctx.accounts.bonding_curve.graduated == GraduationState::Active,
        ErrorCode::InvalidProgramExecutable
    );

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    let global_config = &mut ctx.accounts.global_config;
    let seller = &ctx.accounts.seller;

    require_gt!(tokens_in, 0, ErrorCode::InvalidProgramExecutable);

    let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
    let initial_token_reserves = bonding_curve.virtual_token_reserves;
    let sol_out = calculate_sol_out(tokens_in, initial_sol_reserves, initial_token_reserves)?;

    let token_transfer_accounts = token::Transfer {
        from: ctx.accounts.seller_token_account.to_account_info(),
        to: ctx.accounts.bonding_curve_token_account.to_account_info(),
        authority: seller.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token_transfer_accounts,
    );

    token::transfer(cpi_ctx, tokens_in)?;

    if bonding_curve.to_account_info().lamports() < sol_out {
        return err!(ErrorCode::InvalidProgramExecutable);
    }

    msg!("sol_out: {}", sol_out);
    msg!("tokens_in: {}", tokens_in);
    msg!(
        "Before: bonding_curve lamports: {}",
        bonding_curve.to_account_info().lamports()
    );
    msg!(
        "Before: seller lamports: {}",
        ctx.accounts.seller.to_account_info().lamports()
    );

    bonding_curve.sub_lamports(sol_out)?;
    ctx.accounts.seller.add_lamports(sol_out)?;

    bonding_curve.real_token_reserves = bonding_curve
        .real_token_reserves
        .checked_add(tokens_in)
        .ok_or(PumpError::InvalidRealTokenReserves)?;
    bonding_curve.real_sol_reserves = bonding_curve
        .real_sol_reserves
        .checked_sub(sol_out)
        .ok_or(PumpError::InvalidRealSolReserves)?;
    bonding_curve.virtual_token_reserves = bonding_curve
        .virtual_token_reserves
        .checked_add(tokens_in)
        .ok_or(PumpError::InvalidVirtualTokenReserves)?;
    bonding_curve.virtual_sol_reserves = bonding_curve
        .virtual_sol_reserves
        .checked_sub(sol_out)
        .ok_or(PumpError::InvalidVirtualSolReserves)?;

    global_config.total_volume_sol = global_config
        .total_volume_sol
        .checked_add(sol_out as u128)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(())
}

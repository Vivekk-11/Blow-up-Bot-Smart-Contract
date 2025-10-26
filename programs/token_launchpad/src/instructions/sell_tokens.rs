use anchor_lang::prelude::{program::invoke_signed, system_instruction::transfer, *};
use anchor_spl::token;

use crate::{
    account::sell_tokens::SellTokens, math::calculate_sol_out,
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
    let creator_key = bonding_curve.creator;
    let seeds: &[&[u8]] = &[
        b"bonding-curve",
        creator_key.as_ref(),
        &[bonding_curve.bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

    require_gt!(tokens_in, 0, ErrorCode::InvalidProgramExecutable);

    let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
    let initial_token_reserves = bonding_curve.virtual_token_reserves;
    let sol_out = calculate_sol_out(tokens_in, initial_sol_reserves, initial_token_reserves)?;

    let fee_bps = global_config.sell_fee_bps as u128;
    let sol_128 = sol_out as u128;
    let fee_128 = fee_bps
        .checked_mul(sol_128)
        .ok_or(ErrorCode::InvalidNumericConversion)?
        .checked_div(10_000u128)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let fee = fee_128 as u64;
    let final_sol = sol_out
        .checked_sub(fee)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

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

    if fee > 0 {
        let transfer_fee_ix = transfer(&bonding_curve.key(), &ctx.accounts.treasury.key(), fee);

        invoke_signed(
            &transfer_fee_ix,
            &[
                bonding_curve.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;
    }

    let transfer_sol_ix = transfer(&bonding_curve.key(), &seller.key(), final_sol);

    invoke_signed(
        &transfer_sol_ix,
        &[
            bonding_curve.to_account_info(),
            seller.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    bonding_curve.real_token_reserves = bonding_curve
        .real_token_reserves
        .checked_add(tokens_in)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    bonding_curve.real_sol_reserves = bonding_curve
        .real_sol_reserves
        .checked_sub(sol_out)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    global_config.total_volume_sol = global_config
        .total_volume_sol
        .checked_add(final_sol as u128)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(())
}

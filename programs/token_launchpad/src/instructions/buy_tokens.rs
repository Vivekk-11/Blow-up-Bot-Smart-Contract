use crate::{
    account::buy_tokens::BuyTokens, helpers::graduate::graduate, math::calculate_tokens_out,
};
use anchor_lang::prelude::{
    program::{invoke, invoke_signed},
    system_instruction::transfer,
    *,
};
use anchor_spl::token;

pub fn handler(ctx: Context<BuyTokens>, sol_amount: u64, nonce: u64) -> Result<()> {
    require!(
        !ctx.accounts.bonding_curve.graduated,
        ErrorCode::InvalidProgramExecutable
    );

    {
        let cfg = &mut ctx.accounts.global_config;
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let buyer = &ctx.accounts.buyer;
        let creator_key = bonding_curve.creator;

        let fee_bps = cfg.buy_fee_bps as u128;
        let sol_amount_128 = sol_amount as u128;
        let fee_128 = fee_bps
            .checked_mul(sol_amount_128)
            .ok_or(ErrorCode::InvalidProgramExecutable)?
            .checked_div(10_000u128)
            .ok_or(ErrorCode::InvalidProgramExecutable)?;

        let fee = fee_128 as u64;
        let effective_sol = sol_amount
            .checked_sub(fee)
            .ok_or(ErrorCode::InvalidProgramExecutable)?;

        let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
        let initial_token_reserves = bonding_curve.virtual_token_reserves;
        let tokens_out =
            calculate_tokens_out(effective_sol, initial_sol_reserves, initial_token_reserves)?;

        let transaction_ix = transfer(&buyer.key(), &bonding_curve.key(), sol_amount.clone());

        invoke(
            &transaction_ix,
            &[
                buyer.to_account_info(),
                bonding_curve.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let seeds: &[&[u8]] = &[
            b"bonding-curve",
            creator_key.as_ref(),
            &[bonding_curve.bump],
        ];
        let signer_seeds = &[seeds];

        if fee > 0 {
            let transaction_fee_ix =
                transfer(&bonding_curve.key(), &ctx.accounts.treasury.key(), fee);
            invoke_signed(
                &transaction_fee_ix,
                &[
                    bonding_curve.to_account_info(),
                    ctx.accounts.treasury.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;
        }

        let transfer_token_accounts = token::Transfer {
            from: ctx.accounts.bonding_curve_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: bonding_curve.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_token_accounts,
            signer_seeds,
        );

        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_add(effective_sol)
            .ok_or(ErrorCode::InvalidNumericConversion)?;
        bonding_curve.real_token_reserves = bonding_curve
            .real_token_reserves
            .checked_sub(tokens_out)
            .ok_or(ErrorCode::InvalidNumericConversion)?;
        cfg.total_volume_sol = cfg
            .total_volume_sol
            .checked_add(effective_sol as u128)
            .ok_or(ErrorCode::InvalidNumericConversion)?;

        token::transfer(cpi_ctx, tokens_out)?;
    }

    if ctx.accounts.bonding_curve.real_sol_reserves
        >= ctx.accounts.global_config.graduation_threshold
    {
        graduate(ctx, nonce)?;
    }

    Ok(())
}

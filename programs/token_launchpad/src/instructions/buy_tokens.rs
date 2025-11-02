use crate::{
    account::buy_tokens::BuyTokens, error::PumpError, helpers::graduate::graduate_internal,
    math::calculate_tokens_out, state::bonding_curve::GraduationState,
};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token;

pub fn handler(ctx: Context<BuyTokens>, sol_amount: u64) -> Result<()> {
    if ctx.accounts.bonding_curve.graduated == GraduationState::Pending {
        graduate_internal(ctx)?;
        return Ok(());
    }

    require!(
        ctx.accounts.bonding_curve.graduated == GraduationState::Active,
        PumpError::TokenNotActive
    );

    {
        let cfg = &mut ctx.accounts.global_config;
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let token_mint = ctx.accounts.token_mint.key();
        let creator_key = bonding_curve.creator;

        let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
        let initial_token_reserves = bonding_curve.virtual_token_reserves;
        let tokens_out =
            calculate_tokens_out(sol_amount, initial_sol_reserves, initial_token_reserves)?;

        let seeds: &[&[u8]] = &[
            b"bonding-curve",
            token_mint.as_ref(),
            creator_key.as_ref(),
            &[bonding_curve.bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

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

        msg!(
            "bonding_curve lamports before: {}",
            bonding_curve.to_account_info().lamports()
        );

        let transfer_instruction = system_program::Transfer {
            from: ctx.accounts.buyer.to_account_info(),
            to: bonding_curve.to_account_info(),
        };

        let cpi_ctx1 = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_instruction,
        );

        system_program::transfer(cpi_ctx1, sol_amount)?;

        msg!("sol {}", sol_amount);

        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_add(sol_amount)
            .ok_or(PumpError::InvalidRealSolReserves)?;
        bonding_curve.real_token_reserves = bonding_curve
            .real_token_reserves
            .checked_sub(tokens_out)
            .ok_or(PumpError::InvalidRealTokenReserves)?;
        msg!(
            "Before: virtual_sol_reserves = {}",
            bonding_curve.virtual_sol_reserves
        );
        bonding_curve.virtual_sol_reserves = bonding_curve
            .virtual_sol_reserves
            .checked_add(sol_amount)
            .ok_or(PumpError::InvalidVirtualSolReserves)?;
        msg!(
            "After: virtual_sol_reserves = {}",
            bonding_curve.virtual_sol_reserves
        );
        bonding_curve.virtual_token_reserves = bonding_curve
            .virtual_token_reserves
            .checked_sub(tokens_out)
            .ok_or(PumpError::InvalidVirtualTokenReserves)?;
        cfg.total_volume_sol = cfg
            .total_volume_sol
            .checked_add(sol_amount as u128)
            .ok_or(PumpError::InvalidTotalVolumeSol)?;

        token::transfer(cpi_ctx, tokens_out)?;
    }

    if ctx.accounts.bonding_curve.real_sol_reserves
        >= ctx.accounts.global_config.graduation_threshold
    {
        msg!("graduated!");
        graduate_internal(ctx)?;
    }

    Ok(())
}

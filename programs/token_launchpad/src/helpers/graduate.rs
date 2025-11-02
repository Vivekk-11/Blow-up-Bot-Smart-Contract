use anchor_lang::prelude::*;
use anchor_spl::{associated_token::spl_associated_token_account, token};

use crate::{
    account::buy_tokens::BuyTokens,
    error::PumpError,
    state::{bonding_curve::GraduationState, pool_request::CreatePoolRequestEvent},
};
pub fn graduate_internal(ctx: Context<BuyTokens>) -> Result<()> {
    require!(
        ctx.accounts.bonding_curve.graduated == GraduationState::Active
            || ctx.accounts.bonding_curve.graduated == GraduationState::Pending,
        PumpError::TokenGraduated
    );

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    let token_mint = ctx.accounts.token_mint.key();
    let creator_key = bonding_curve.creator;

    let seeds_raw: &[&[u8]] = &[
        b"bonding-curve",
        token_mint.as_ref(),
        creator_key.as_ref(),
        &[bonding_curve.bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&seeds_raw[..]];

    let expected_relayer_token_ata = spl_associated_token_account::get_associated_token_address(
        &ctx.accounts.relayer.key(),
        &ctx.accounts.token_mint.key(),
    );

    require_keys_eq!(
        expected_relayer_token_ata,
        ctx.accounts.relayer_token_account.key(),
        ErrorCode::InvalidProgramExecutable
    );

    let token_amount: u64 = bonding_curve.real_token_reserves;

    if token_amount > 0 {
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.bonding_curve_token_account.to_account_info(),
            to: ctx.accounts.relayer_token_account.to_account_info(),
            authority: bonding_curve.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, token_amount)?;
        bonding_curve.real_token_reserves = 0;
    }

    let expected_relayer_wsol_ata = spl_associated_token_account::get_associated_token_address(
        &ctx.accounts.relayer.key(),
        &ctx.accounts.wsol_mint_account.key(),
    );
    require_keys_eq!(
        expected_relayer_wsol_ata,
        ctx.accounts.relayer_wsol_account.key(),
        ErrorCode::InvalidProgramExecutable
    );

    let total_wrap = bonding_curve.real_sol_reserves;
    if total_wrap > 0 {
        bonding_curve.sub_lamports(total_wrap)?;
        ctx.accounts.relayer_wsol_account.add_lamports(total_wrap)?;
        bonding_curve.real_sol_reserves = 0;
    }

    let actual_token_amount = ctx.accounts.relayer_token_account.amount;

    let clock = Clock::get()?;
    emit!(CreatePoolRequestEvent {
        bonding_curve: bonding_curve.key(),
        token_mint: ctx.accounts.token_mint.key(),
        wsol_mint: ctx.accounts.wsol_mint_account.key(),
        wsol_ata: ctx.accounts.relayer_wsol_account.key(),
        token_amount: actual_token_amount,
        token_ata: ctx.accounts.relayer_token_account.key(),
        wsol_amount: total_wrap,
        timestamp: clock.unix_timestamp,
        creator: bonding_curve.creator
    });

    bonding_curve.graduated = GraduationState::Pending;

    Ok(())
}

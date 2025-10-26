use crate::state::graduate::GraduatedEvent;
use crate::{account::graduate::Graduate, state::bonding_curve::GraduationState};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn};

pub fn handler(ctx: Context<Graduate>, pool: Pubkey) -> Result<()> {
    let bonding_curve = &mut ctx.accounts.bonding_curve;
    let lp_amount = ctx.accounts.lp_token_account.amount;

    let seeds: &[&[u8]] = &[
        b"bonding-curve",
        bonding_curve.creator.as_ref(),
        &[bonding_curve.bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

    let cpi_accounts = Burn {
        mint: ctx.accounts.lp_mint.to_account_info(),
        from: ctx.accounts.lp_token_account.to_account_info(),
        authority: bonding_curve.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    token::burn(cpi_ctx, lp_amount)?;

    bonding_curve.graduated = GraduationState::Graduated;
    bonding_curve.pool = Some(pool);

    let ts = Clock::get()?.unix_timestamp;
    emit!(GraduatedEvent {
        mint: bonding_curve.token_mint,
        authority: bonding_curve.key(),
        timestamp: ts,
        pool
    });

    Ok(())
}

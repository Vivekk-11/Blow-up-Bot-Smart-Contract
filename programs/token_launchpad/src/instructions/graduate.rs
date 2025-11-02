use crate::state::graduate::GraduatedEvent;
use crate::{account::graduate::Graduate, state::bonding_curve::GraduationState};
use anchor_lang::prelude::*;

pub fn handler(ctx: Context<Graduate>, pool: Pubkey) -> Result<()> {
    let bonding_curve = &mut ctx.accounts.bonding_curve;

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

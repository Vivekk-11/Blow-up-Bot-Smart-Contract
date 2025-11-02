use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::state::{
    bonding_curve::{BondingCurve},
};

#[derive(Accounts)]
pub struct Graduate<'info> {
    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"bonding-curve", token_mint.key().as_ref(), bonding_curve.creator.as_ref()],
        bump = bonding_curve.bump,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(mut)]
    pub relayer: Signer<'info>,
}

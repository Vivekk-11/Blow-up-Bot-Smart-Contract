use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::{
    bonding_curve::{BondingCurve, GraduationState},
    config::GlobalConfig,
};

#[derive(Accounts)]
pub struct Graduate<'info> {
    #[account(
        mut,
        seeds = [b"bonding-curve", bonding_curve.creator.as_ref()],
        bump = bonding_curve.bump,
        constraint = bonding_curve.graduated == GraduationState::Pending @ ErrorCode::InvalidProgramExecutable
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
        mut,
        constraint = relayer.key() == global_config.allowed_relayer @ ErrorCode::InvalidProgramExecutable
    )]
    pub relayer: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = bonding_curve,
        constraint = lp_token_account.amount > 0 @ ErrorCode::InvalidProgramExecutable,
    )]
    pub lp_token_account: Account<'info, TokenAccount>,

    pub global_config: Account<'info, GlobalConfig>,
    pub token_program: Program<'info, Token>,
}

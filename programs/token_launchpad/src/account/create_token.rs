use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::{
    bonding_curve::BondingCurve,
    config::{GlobalConfig, TOKEN_DECIMALS},
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK: Treasury is a raw `AccountInfo`
    #[account(
        mut,
        constraint = treasury.key() == global_config.treasury @ ErrorCode::InvalidProgramExecutable
    )]
    pub treasury: AccountInfo<'info>,

    #[account(
        init,
        payer = creator, 
        mint::decimals = TOKEN_DECIMALS,
        mint::authority = bonding_curve
    )]
    pub token_mint: Account<'info, Mint>,

    #[account(
        init, 
        payer = creator,
        space = 8 + std::mem::size_of::<BondingCurve>(),
        seeds = [b"bonding-curve", token_mint.key().as_ref(), creator.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
    init_if_needed,
    payer = creator,
    associated_token::mint = token_mint,
    associated_token::authority = bonding_curve
)]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    /// CHECK: SOMETHING
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: SOMETHING
    pub token_metadata_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

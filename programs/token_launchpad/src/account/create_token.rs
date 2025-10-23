use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::{bonding_curve::BondingCurve, config::{GlobalConfig, TOKEN_DECIMALS}};

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

    #[account(
        mut,
        constraint = treasury.key() == global_config.treasury @ ErrorCode::InvalidProgramExecutable
    )]
    pub treasury: AccountInfo<'info>,

    #[account(
        init, 
        payer = creator,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1,
        seeds = [b"bonding-curve", creator.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

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
        token::mint = token_mint,
        token::authority = bonding_curve
    )]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_metadata_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

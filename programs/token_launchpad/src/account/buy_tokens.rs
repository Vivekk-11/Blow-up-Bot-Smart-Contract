use anchor_lang::prelude::*;
use anchor_spl::{
    token::Mint,
    token::{Token, TokenAccount},
};

use crate::state::{bonding_curve::BondingCurve, config::GlobalConfig, pool_request::PoolRequest};

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

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
        mut,
        seeds = [b"bonding-curve", bonding_curve.creator.as_ref()],
        bump = bonding_curve.bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(mut)]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    /// CHECK: SOMETHING
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,

    /// CHECK: SOMETHING
    #[account(mut)]
    pub wsol_temp_token_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub liquidity_token_account: Account<'info, TokenAccount>,

    /// CHECK: SOMETHING
    #[account(mut)]
    pub pool_account: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + 32*5 + 8*3 + 1 + 1,
        seeds = [b"pool-request", bonding_curve.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump
    )]
    pub pool_request: Account<'info, PoolRequest>,

    pub wsol_mint_account: Account<'info, Mint>,

    /// CHECK: SOMETHING
    pub associated_token_program: UncheckedAccount<'info>,

    /// CHECK: SOMETHING
    pub token_metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

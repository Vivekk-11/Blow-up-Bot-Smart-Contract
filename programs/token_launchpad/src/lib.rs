#![allow(unexpected_cfgs)]

mod state;
use state::config::*;
mod account;
use account::buy_tokens::*;
use account::create_token::*;
use account::sell_tokens::*;
mod helpers;
mod instructions;
mod math;
use anchor_lang::prelude::*;


declare_id!("3fWbxmqbqFzvewGqJ9iNqyC22RuFhJ8Yof1nEWbgHimF");

#[program]
pub mod token_launchpad {
    use super::*;

    pub fn create_token(
        ctx: Context<CreateToken>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::create_tokens::handler(ctx, name, symbol, uri)
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, sol_amount: u64, nonce: u64) -> Result<()> {
        instructions::buy_tokens::handler(ctx, sol_amount, nonce)
    }

    pub fn sell_tokens(ctx: Context<SellTokens>, tokens_in: u64) -> Result<()> {
        instructions::sell_tokens::handler(ctx, tokens_in)
    }
}

pub fn initialize_global_config(
    ctx: Context<InitializeGlobalConfig>,
    treasury: Pubkey,
) -> Result<()> {
    let cfg = &mut ctx.accounts.global_config;

    require!(
        treasury != Pubkey::default(),
        ErrorCode::InvalidProgramExecutable
    );

    cfg.authority = ctx.accounts.admin.key();
    cfg.treasury = treasury;

    require!(
        DEFAULT_BUY_FEE_BPS <= MAX_BUY_FEE_BPS,
        ErrorCode::InvalidProgramExecutable
    );
    require!(
        DEFAULT_SELL_FEE_BPS <= MAX_SELL_FEE_BPS,
        ErrorCode::InvalidProgramExecutable
    );
    require!(
        DEFAULT_CREATION_FEE <= MAX_CREATION_FEE,
        ErrorCode::InvalidProgramExecutable
    );

    cfg.buy_fee_bps = DEFAULT_BUY_FEE_BPS;
    cfg.sell_fee_bps = DEFAULT_SELL_FEE_BPS;
    cfg.creation_fee = DEFAULT_CREATION_FEE;
    cfg.graduation_threshold = DEFAULT_GRADUATION_THRESHOLD;

    cfg.total_tokens_created = 0;
    cfg.total_volume_sol = 0u128;
    cfg.paused = false;

    cfg.bump = ctx.bumps.global_config;

    Ok(())
}

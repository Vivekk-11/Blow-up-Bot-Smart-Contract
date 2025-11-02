#![allow(unexpected_cfgs)]

mod account;
mod state;
use account::buy_tokens::*;
use account::create_token::*;
use account::global_config::*;
use account::graduate::*;
use account::sell_tokens::*;
mod helpers;
use account::delete_program_data::*;
mod instructions;
mod math;
use anchor_lang::prelude::*;
mod error;

declare_id!("HzPQomW78T8mRKMiXaCKEKLgXWvzMxLCRjeffgxfBz3r");

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
    pub fn init_global_config(
        ctx: Context<InitializeGlobalConfig>,
        treasury: Pubkey,
        graduation_threshold: u64,
    ) -> Result<()> {
        instructions::configs::handler(ctx, treasury, graduation_threshold)
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, sol_amount: u64) -> Result<()> {
        instructions::buy_tokens::handler(ctx, sol_amount)
    }

    pub fn sell_tokens(ctx: Context<SellTokens>, tokens_in: u64) -> Result<()> {
        instructions::sell_tokens::handler(ctx, tokens_in)
    }

    pub fn graduate(ctx: Context<Graduate>, pool: Pubkey) -> Result<()> {
        instructions::graduate::handler(ctx, pool)
    }

    pub fn delete_data(_ctx: Context<DeleteData>) -> Result<()> {
        Ok(())
    }
}

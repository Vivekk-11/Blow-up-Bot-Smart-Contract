use crate::{
    account::global_config::InitializeGlobalConfig,
    error::PumpError,
    state::config::{DEFAULT_BUY_FEE_BPS, DEFAULT_CREATION_FEE, DEFAULT_SELL_FEE_BPS},
};
use anchor_lang::prelude::*;

pub fn handler(
    ctx: Context<InitializeGlobalConfig>,
    treasury: Pubkey,
    graduation_threshold: u64,
) -> Result<()> {
    let cfg = &mut ctx.accounts.global_config;

    require!(treasury != Pubkey::default(), PumpError::InvalidTreasury);

    cfg.authority = ctx.accounts.admin.key();
    cfg.treasury = treasury;
    cfg.buy_fee_bps = DEFAULT_BUY_FEE_BPS;
    cfg.sell_fee_bps = DEFAULT_SELL_FEE_BPS;
    cfg.creation_fee = DEFAULT_CREATION_FEE;
    cfg.graduation_threshold = graduation_threshold;
    cfg.total_tokens_created = 0;
    cfg.total_volume_sol = 0;
    cfg.paused = false;
    cfg.bump = ctx.bumps.global_config;

    Ok(())
}

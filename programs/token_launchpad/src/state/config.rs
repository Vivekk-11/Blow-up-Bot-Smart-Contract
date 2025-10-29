use anchor_lang::prelude::*;

pub const INITIAL_VIRTUAL_SOL_RESERVES: u64 = 1_000_000_000_000; // 1000 SOL in lamports
pub const INITIAL_VIRTUAL_TOKEN_RESERVES: u64 = 1_000_000_000_000_000; // 1 billion tokens with 6 decimals (1_000_000_000 * 10^6)
pub const REAL_TOKEN_RESERVES: u64 = 1_000_000_000_000_000_000; // 1 trillion tokens with 6 decimals

pub const TOKEN_DECIMALS: u8 = 6;

pub const DEFAULT_BUY_FEE_BPS: u16 = 100;
pub const DEFAULT_SELL_FEE_BPS: u16 = 100;
pub const DEFAULT_CREATION_FEE: u64 = 20_000_000;
pub const DEFAULT_GRADUATION_THRESHOLD: u64 = 85_000_000_000;

// pub const MAX_BUY_FEE_BPS: u16 = 1000;
// pub const MAX_SELL_FEE_BPS: u16 = 1000;
// pub const MAX_CREATION_FEE: u64 = 100_000_000;

#[account]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub buy_fee_bps: u16,
    pub sell_fee_bps: u16,
    pub creation_fee: u64,
    pub graduation_threshold: u64,
    pub total_tokens_created: u64,
    pub total_volume_sol: u128,
    pub allowed_relayer: Pubkey,
    pub paused: bool,
    pub bump: u8,
}

use anchor_lang::prelude::*;

#[account]
pub struct BondingCurve {
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub graduated: bool,
    pub bump: u8,
}

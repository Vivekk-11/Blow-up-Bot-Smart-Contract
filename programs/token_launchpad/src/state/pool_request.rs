use anchor_lang::prelude::*;

#[account]
pub struct PoolRequest {
    pub bonding_curve: Pubkey,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub wsol_ata: Pubkey,
    pub token_amount: u64,
    pub wsol_amount: u64,
    pub nonce: u64,
    pub fulfilled: bool,
    pub pool_pubkey: Pubkey,
    pub bump: u8,
}

#[event]
pub struct CreatePoolRequestEvent {
    pub bonding_curve: Pubkey,
    pub token_mint: Pubkey,
    pub wsol_ata: Pubkey,
    pub token_amount: u64,
    pub wsol_amount: u64,
    pub nonce: u64,
    pub timestamp: i64,
}

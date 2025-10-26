use anchor_lang::prelude::*;

#[event]
pub struct CreatePoolRequestEvent {
    pub bonding_curve: Pubkey,
    pub token_mint: Pubkey,
    pub wsol_ata: Pubkey,
    pub token_amount: u64,
    pub wsol_amount: u64,
    pub creator: Pubkey,
    pub timestamp: i64,
}

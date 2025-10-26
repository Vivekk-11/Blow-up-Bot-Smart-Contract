use anchor_lang::prelude::*;

#[event]
pub struct GraduatedEvent {
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

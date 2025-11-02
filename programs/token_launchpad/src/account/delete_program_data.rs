use anchor_lang::prelude::*;

use crate::state::bonding_curve::BondingCurve;

#[derive(Accounts)]
pub struct DeleteData<'info> {
    #[account(mut, close = receiver)] 
    pub my_data_account: Account<'info, BondingCurve>, 

    #[account(mut)]
    pub receiver: Signer<'info>,
}
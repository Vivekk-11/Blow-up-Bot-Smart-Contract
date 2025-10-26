use anchor_lang::prelude::program::invoke;
use anchor_lang::prelude::system_instruction::transfer;
use anchor_lang::prelude::*;
use anchor_spl::metadata::mpl_token_metadata::types::DataV2;
use anchor_spl::metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3};
use anchor_spl::token;
use anchor_spl::{metadata::mpl_token_metadata, token::MintTo};

use crate::state::bonding_curve::GraduationState;
use crate::{
    account::create_token::CreateToken,
    state::config::{
        INITIAL_VIRTUAL_SOL_RESERVES, INITIAL_VIRTUAL_TOKEN_RESERVES, REAL_TOKEN_RESERVES,
        TOKEN_TOTAL_SUPPLY,
    },
};

pub fn handler(ctx: Context<CreateToken>, name: String, symbol: String, uri: String) -> Result<()> {
    let bonding_curve = &mut ctx.accounts.bonding_curve;
    let cfg = &mut ctx.accounts.global_config;

    bonding_curve.creator = ctx.accounts.creator.key();
    bonding_curve.token_mint = ctx.accounts.token_mint.key();
    bonding_curve.virtual_sol_reserves = INITIAL_VIRTUAL_SOL_RESERVES;
    bonding_curve.virtual_token_reserves = INITIAL_VIRTUAL_TOKEN_RESERVES;
    bonding_curve.real_sol_reserves = 0;
    bonding_curve.real_token_reserves = REAL_TOKEN_RESERVES;
    bonding_curve.graduated = GraduationState::Active;
    bonding_curve.bump = ctx.bumps.bonding_curve;

    let seeds: &[&[u8]] = &[
        b"bonding-curve",
        ctx.accounts.creator.key.as_ref(),
        &[bonding_curve.bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.token_mint.to_account_info(),
        to: ctx.accounts.bonding_curve_token_account.to_account_info(),
        authority: bonding_curve.to_account_info(),
    };

    let cpi_cxt = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    token::mint_to(cpi_cxt, TOKEN_TOTAL_SUPPLY)?;

    let (metadata_pda, _metadata_bump) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID.as_ref(),
            ctx.accounts.token_mint.key().as_ref(),
        ],
        &mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID,
    );

    require_keys_eq!(
        metadata_pda,
        ctx.accounts.metadata_account.key(),
        ErrorCode::InvalidProgramExecutable
    );

    let data = DataV2 {
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        uses: None,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
    };

    let metadata_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata_account.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
            mint_authority: ctx.accounts.bonding_curve.to_account_info(),
            payer: ctx.accounts.creator.to_account_info(),
            update_authority: ctx.accounts.bonding_curve.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
        signer_seeds,
    );

    create_metadata_accounts_v3(metadata_ctx, data, false, false, None)?;

    let create_fee_ix = transfer(&ctx.accounts.creator.key(), &cfg.treasury, cfg.creation_fee);

    invoke(
        &create_fee_ix,
        &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.treasury.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    cfg.total_tokens_created = cfg
        .total_tokens_created
        .checked_add(1)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(())
}

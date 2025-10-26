use anchor_lang::prelude::{program::invoke_signed, program_pack::Pack, *};
use anchor_spl::{associated_token::spl_associated_token_account, token, token::spl_token};

use crate::{account::buy_tokens::BuyTokens, state::pool_request::CreatePoolRequestEvent};

pub fn graduate_internal(ctx: Context<BuyTokens>, nonce: u64) -> Result<()> {
    // TODO: check if atas exist or not... If yes, then wrap up everything and then emit an event. If atas don't exist,
    // create atas, and then emit an event!

    require!(
        !ctx.accounts.bonding_curve.graduated,
        ErrorCode::InvalidProgramExecutable
    );

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    let creator_key = bonding_curve.creator;

    let seeds_raw: &[&[u8]] = &[
        b"bonding-curve",
        creator_key.as_ref(),
        &[bonding_curve.bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&seeds_raw[..]];

    let rent = Rent::get()?;
    let ata_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let token_amount: u64 = bonding_curve.real_token_reserves;

    let wsol_temp_info = ctx.accounts.wsol_temp_token_account.to_account_info();
    let expected_wsol_ata = spl_associated_token_account::get_associated_token_address(
        &bonding_curve.key(),
        &ctx.accounts.wsol_mint_account.key(),
    );

    require_keys_eq!(
        expected_wsol_ata,
        ctx.accounts.wsol_temp_token_account.key(),
        ErrorCode::InvalidProgramExecutable
    );

    if wsol_temp_info.data_is_empty() {
        let create_wsol_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &bonding_curve.key(),
                &bonding_curve.key(),
                &ctx.accounts.wsol_mint_account.key(),
                &spl_token::id(),
            );

        invoke_signed(
            &create_wsol_ix,
            &[
                bonding_curve.to_account_info(),
                wsol_temp_info.clone(),
                bonding_curve.to_account_info(),
                ctx.accounts.wsol_mint_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.associated_token_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_sub(ata_rent)
            .unwrap_or(0);
    } else {
        require_keys_eq!(
            spl_token::id(),
            *wsol_temp_info.owner,
            ErrorCode::InvalidProgramExecutable
        );
        let ata_data = spl_token::state::Account::unpack(&wsol_temp_info.data.borrow())?;
        require_keys_eq!(
            ata_data.owner,
            bonding_curve.key(),
            ErrorCode::InvalidProgramExecutable
        );
        require_keys_eq!(
            ata_data.mint,
            ctx.accounts.wsol_mint_account.key(),
            ErrorCode::InvalidProgramExecutable
        );
    }

    let total_wrap = bonding_curve.real_sol_reserves;
    let mut actual_wrap: u64 = 0;

    if total_wrap > 0 {
        let bonding_curve_size = bonding_curve.to_account_info().data_len();
        let min_keep = rent.minimum_balance(bonding_curve_size);
        let pda_lamports = bonding_curve.to_account_info().lamports();
        let max_transferrable = pda_lamports.saturating_sub(min_keep);

        let amount_to_wrap = core::cmp::min(total_wrap, max_transferrable);

        if amount_to_wrap > 0 {
            let transfer_ix = system_instruction::transfer(
                &bonding_curve.key(),
                &expected_wsol_ata,
                amount_to_wrap,
            );
            invoke_signed(
                &transfer_ix,
                &[
                    bonding_curve.to_account_info(),
                    ctx.accounts.wsol_temp_token_account.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;

            let sync_ix =
                spl_token::instruction::sync_native(&spl_token::id(), &expected_wsol_ata)?;
            invoke_signed(
                &sync_ix,
                &[
                    ctx.accounts.wsol_temp_token_account.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                ],
                signer_seeds,
            )?;

            bonding_curve.real_sol_reserves = bonding_curve
                .real_sol_reserves
                .checked_sub(amount_to_wrap)
                .unwrap_or(0);

            actual_wrap = amount_to_wrap;
        }
    }

    let liq_ata_info = ctx.accounts.liquidity_token_account.to_account_info();
    let expected_liq_ata = spl_associated_token_account::get_associated_token_address(
        &bonding_curve.key(),
        &ctx.accounts.token_mint.key(),
    );

    require_keys_eq!(
        expected_liq_ata,
        liq_ata_info.key(),
        ErrorCode::InvalidProgramExecutable
    );

    if liq_ata_info.data_is_empty() {
        let create_liq_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &bonding_curve.key(),
                &bonding_curve.key(),
                &ctx.accounts.token_mint.key(),
                &spl_token::id(),
            );

        invoke_signed(
            &create_liq_ix,
            &[
                bonding_curve.to_account_info(),
                liq_ata_info.clone(),
                bonding_curve.to_account_info(),
                ctx.accounts.token_mint.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.associated_token_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_sub(ata_rent)
            .unwrap_or(0);
    } else {
        require_keys_eq!(
            spl_token::id(),
            *liq_ata_info.owner,
            ErrorCode::InvalidProgramExecutable
        );
        let liq_data = spl_token::state::Account::unpack(&liq_ata_info.data.borrow())?;
        require_keys_eq!(
            liq_data.owner,
            bonding_curve.key(),
            ErrorCode::InvalidProgramExecutable
        );
        require_keys_eq!(
            liq_data.mint,
            ctx.accounts.token_mint.key(),
            ErrorCode::InvalidProgramExecutable
        );
    }

    if token_amount > 0 {
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.bonding_curve_token_account.to_account_info(),
            to: ctx.accounts.liquidity_token_account.to_account_info(),
            authority: bonding_curve.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, token_amount)?;
        bonding_curve.real_token_reserves = bonding_curve
            .real_token_reserves
            .checked_sub(token_amount)
            .unwrap_or(0);
    }

    bonding_curve.graduated = true;

    let pool_request = &mut ctx.accounts.pool_request;
    pool_request.bonding_curve = bonding_curve.key();
    pool_request.creator = bonding_curve.creator;
    pool_request.token_mint = ctx.accounts.token_mint.key();
    pool_request.wsol_ata = ctx.accounts.wsol_temp_token_account.key();
    pool_request.token_amount = token_amount;
    pool_request.wsol_amount = actual_wrap;
    pool_request.nonce = nonce;
    pool_request.fulfilled = false;
    pool_request.pool_pubkey = Pubkey::default();
    pool_request.bump = ctx.bumps.pool_request;

    let clock = Clock::get()?;
    emit!(CreatePoolRequestEvent {
        bonding_curve: bonding_curve.key(),
        token_mint: ctx.accounts.token_mint.key(),
        wsol_ata: ctx.accounts.wsol_temp_token_account.key(),
        token_amount,
        wsol_amount: actual_wrap,
        nonce,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

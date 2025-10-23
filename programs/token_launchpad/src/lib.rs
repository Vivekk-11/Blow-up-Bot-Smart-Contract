mod state;

use state::config::*;

use anchor_lang::{
    prelude::{program_pack::Pack, *},
    solana_program::{
        self,
        program::{invoke, invoke_signed},
        system_instruction::transfer,
    },
};
use anchor_spl::metadata::{
    create_metadata_accounts_v3,
    mpl_token_metadata::{self, types::DataV2},
    CreateMetadataAccountsV3,
};
use anchor_spl::{
    associated_token::spl_associated_token_account::{self},
    token::{
        self,
        spl_token::{self},
        Mint, MintTo, Token, TokenAccount,
    },
};

use crate::state::pool_request::{CreatePoolRequestEvent, PoolRequest};

declare_id!("11111111111111111111111111111111");

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

#[program]
pub mod token_launchpad {

    use super::*;

    pub fn create_token(
        ctx: Context<CreateToken>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        // TODO: create the mint account
        // TODO: create the treasury

        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let cfg = &mut ctx.accounts.global_config;

        bonding_curve.creator = ctx.accounts.creator.key();
        bonding_curve.token_mint = ctx.accounts.token_mint.key();
        bonding_curve.virtual_sol_reserves = INITIAL_VIRTUAL_SOL_RESERVES;
        bonding_curve.virtual_token_reserves = INITIAL_VIRTUAL_TOKEN_RESERVES;
        bonding_curve.real_sol_reserves = 0;
        bonding_curve.real_token_reserves = REAL_TOKEN_RESERVES;
        bonding_curve.graduated = false;
        bonding_curve.bump = ctx.bumps.bonding_curve;

        let seeds: &[&[u8]] = &[
            b"bonding-curve",
            ctx.accounts.creator.key.as_ref(),
            &[bonding_curve.bump],
        ];

        let signer_seeds = &[seeds];

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

    pub fn buy_tokens(ctx: Context<BuyTokens>, sol_amount: u64, nonce: u64) -> Result<()> {
        require!(
            !ctx.accounts.bonding_curve.graduated,
            ErrorCode::InvalidProgramExecutable
        );

        {
            let cfg = &mut ctx.accounts.global_config;
            let bonding_curve = &mut ctx.accounts.bonding_curve;
            let buyer = &ctx.accounts.buyer;
            let creator_key = bonding_curve.creator;

            let fee_bps = cfg.buy_fee_bps as u128;
            let sol_amount_128 = sol_amount as u128;
            let fee_128 = fee_bps
                .checked_mul(sol_amount_128)
                .ok_or(ErrorCode::InvalidProgramExecutable)?
                .checked_div(10_000u128)
                .ok_or(ErrorCode::InvalidProgramExecutable)?;

            let fee = fee_128 as u64;
            let effective_sol = sol_amount
                .checked_sub(fee)
                .ok_or(ErrorCode::InvalidProgramExecutable)?;

            let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
            let initial_token_reserves = bonding_curve.virtual_token_reserves;
            let tokens_out =
                calculate_tokens_out(effective_sol, initial_sol_reserves, initial_token_reserves)?;

            let transaction_ix = transfer(&buyer.key(), &bonding_curve.key(), sol_amount.clone());

            invoke(
                &transaction_ix,
                &[
                    buyer.to_account_info(),
                    bonding_curve.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;

            let seeds: &[&[u8]] = &[
                b"bonding-curve",
                creator_key.as_ref(),
                &[bonding_curve.bump],
            ];
            let signer_seeds = &[seeds];

            if fee > 0 {
                let transaction_fee_ix =
                    transfer(&bonding_curve.key(), &ctx.accounts.treasury.key(), fee);
                invoke_signed(
                    &transaction_fee_ix,
                    &[
                        bonding_curve.to_account_info(),
                        ctx.accounts.treasury.to_account_info(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    signer_seeds,
                )?;
            }

            let transfer_token_accounts = token::Transfer {
                from: ctx.accounts.bonding_curve_token_account.to_account_info(),
                to: ctx.accounts.buyer_token_account.to_account_info(),
                authority: bonding_curve.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_token_accounts,
                signer_seeds,
            );

            bonding_curve.real_sol_reserves = bonding_curve
                .real_sol_reserves
                .checked_add(effective_sol)
                .ok_or(ErrorCode::InvalidNumericConversion)?;
            bonding_curve.real_token_reserves = bonding_curve
                .real_token_reserves
                .checked_sub(tokens_out)
                .ok_or(ErrorCode::InvalidNumericConversion)?;
            cfg.total_volume_sol = cfg
                .total_volume_sol
                .checked_add(effective_sol as u128)
                .ok_or(ErrorCode::InvalidNumericConversion)?;

            token::transfer(cpi_ctx, tokens_out)?;
        }

        if ctx.accounts.bonding_curve.real_sol_reserves
            >= ctx.accounts.global_config.graduation_threshold
        {
            graduate(ctx, nonce)?;
        }

        Ok(())
    }

    pub fn sell_tokens(ctx: Context<SellTokens>, tokens_in: u64) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let global_config = &mut ctx.accounts.global_config;
        let seller = &ctx.accounts.seller;
        let creator_key = bonding_curve.creator;
        let seeds: &[&[u8]] = &[
            b"bonding-curve",
            creator_key.as_ref(),
            &[bonding_curve.bump],
        ];
        let signer_seeds = &[seeds];

        require_gt!(tokens_in, 0, ErrorCode::InvalidProgramExecutable);

        let initial_sol_reserves = bonding_curve.virtual_sol_reserves;
        let initial_token_reserves = bonding_curve.virtual_token_reserves;
        let sol_out = calculate_sol_out(tokens_in, initial_sol_reserves, initial_token_reserves)?;

        let fee_bps = global_config.sell_fee_bps as u128;
        let sol_128 = sol_out as u128;
        let fee_128 = fee_bps
            .checked_mul(sol_128)
            .ok_or(ErrorCode::InvalidNumericConversion)?
            .checked_div(10_000u128)
            .ok_or(ErrorCode::InvalidNumericConversion)?;
        let fee = fee_128 as u64;
        let final_sol = sol_out
            .checked_sub(fee)
            .ok_or(ErrorCode::InvalidNumericConversion)?;

        let token_transfer_accounts = token::Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.bonding_curve_token_account.to_account_info(),
            authority: seller.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_transfer_accounts,
        );

        token::transfer(cpi_ctx, tokens_in)?;

        if bonding_curve.to_account_info().lamports() < sol_out {
            return err!(ErrorCode::InvalidProgramExecutable);
        }

        if fee > 0 {
            let transfer_fee_ix = transfer(&bonding_curve.key(), &ctx.accounts.treasury.key(), fee);

            invoke_signed(
                &transfer_fee_ix,
                &[
                    bonding_curve.to_account_info(),
                    ctx.accounts.treasury.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;
        }

        let transfer_sol_ix = transfer(&bonding_curve.key(), &seller.key(), final_sol);

        invoke_signed(
            &transfer_sol_ix,
            &[
                bonding_curve.to_account_info(),
                seller.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        bonding_curve.real_token_reserves = bonding_curve
            .real_token_reserves
            .checked_add(tokens_in)
            .ok_or(ErrorCode::InvalidNumericConversion)?;
        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_sub(sol_out)
            .ok_or(ErrorCode::InvalidNumericConversion)?;

        global_config.total_volume_sol = global_config
            .total_volume_sol
            .checked_add(final_sol as u128)
            .ok_or(ErrorCode::InvalidNumericConversion)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        constraint = treasury.key() == global_config.treasury @ ErrorCode::InvalidProgramExecutable
    )]
    pub treasury: AccountInfo<'info>,

    #[account(
        init, 
        payer = creator,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1,
        seeds = [b"bonding-curve", creator.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
        init,
        payer = creator, 
        mint::decimals = TOKEN_DECIMALS,
        mint::authority = bonding_curve
    )]
    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        token::mint = token_mint,
        token::authority = bonding_curve
    )]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_metadata_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"global-config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        constraint = treasury.key() == global_config.treasury @ ErrorCode::InvalidProgramExecutable
    )]
    pub treasury: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"bonding-curve", bonding_curve.creator.as_ref()],
        bump = bonding_curve.bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(mut)]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub wsol_temp_token_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub liquidity_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub pool_account: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + 32*5 + 8*3 + 1 + 1,
        seeds = [b"pool-request", bonding_curve.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump
    )]
    pub pool_request: Account<'info, PoolRequest>,

    pub wsol_mint_account: Account<'info, Mint>,

    pub associated_token_program: UncheckedAccount<'info>,
    pub token_metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SellTokens<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bonding-curve", bonding_curve.creator.as_ref()],
        bump = bonding_curve.bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
        mut,
        seeds = [b"global-config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        constraint = treasury.key() == global_config.treasury @ ErrorCode::InvalidProgramExecutable
    )]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn calculate_tokens_out(
    sol_amount: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    let k = initial_sol_reserves
        .checked_mul(initial_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_sol_reserves = initial_sol_reserves
        .checked_add(sol_amount)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_token_reserves = k
        .checked_div(new_sol_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let tokens_out = initial_token_reserves
        .checked_sub(new_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(tokens_out)
}

pub fn calculate_sol_out(
    tokens_in: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> Result<u64> {
    let k = initial_sol_reserves
        .checked_mul(initial_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_token_reserves = initial_token_reserves
        .checked_add(tokens_in)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let new_sol_reserves = k
        .checked_div(new_token_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;
    let sol_out = initial_sol_reserves
        .checked_sub(new_sol_reserves)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

    Ok(sol_out)
}

pub fn initialize_global_config(
    ctx: Context<InitializeGlobalConfig>,
    treasury: Pubkey,
) -> Result<()> {
    let cfg = &mut ctx.accounts.global_config;

    require!(
        treasury != Pubkey::default(),
        ErrorCode::InvalidProgramExecutable
    );

    cfg.authority = ctx.accounts.admin.key();
    cfg.treasury = treasury;

    require!(
        DEFAULT_BUY_FEE_BPS <= MAX_BUY_FEE_BPS,
        ErrorCode::InvalidProgramExecutable
    );
    require!(
        DEFAULT_SELL_FEE_BPS <= MAX_SELL_FEE_BPS,
        ErrorCode::InvalidProgramExecutable
    );
    require!(
        DEFAULT_CREATION_FEE <= MAX_CREATION_FEE,
        ErrorCode::InvalidProgramExecutable
    );

    cfg.buy_fee_bps = DEFAULT_BUY_FEE_BPS;
    cfg.sell_fee_bps = DEFAULT_SELL_FEE_BPS;
    cfg.creation_fee = DEFAULT_CREATION_FEE;
    cfg.graduation_threshold = DEFAULT_GRADUATION_THRESHOLD;

    cfg.total_tokens_created = 0;
    cfg.total_volume_sol = 0u128;
    cfg.paused = false;

    cfg.bump = ctx.bumps.global_config;

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGlobalConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = GlobalConfig::LEN,
        seeds = [b"global-config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

pub fn graduate(ctx: Context<BuyTokens>, nonce: u64) -> Result<()> {
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
        require_keys_eq!(ata_data.owner, bonding_curve.key(), ErrorCode::InvalidProgramExecutable);
        require_keys_eq!(ata_data.mint, ctx.accounts.wsol_mint_account.key(), ErrorCode::InvalidProgramExecutable);
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

            let sync_ix = spl_token::instruction::sync_native(&spl_token::id(), &expected_wsol_ata)?;
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

    require_keys_eq!(expected_liq_ata, liq_ata_info.key(), ErrorCode::InvalidProgramExecutable);

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
        require_keys_eq!(spl_token::id(), *liq_ata_info.owner, ErrorCode::InvalidProgramExecutable);
        let liq_data = spl_token::state::Account::unpack(&liq_ata_info.data.borrow())?;
        require_keys_eq!(liq_data.owner, bonding_curve.key(), ErrorCode::InvalidProgramExecutable);
        require_keys_eq!(liq_data.mint, ctx.accounts.token_mint.key(), ErrorCode::InvalidProgramExecutable);
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

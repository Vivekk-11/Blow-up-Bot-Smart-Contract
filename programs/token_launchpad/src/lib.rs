mod state;
use state::config::*;

use anchor_lang::{solana_program::{system_instruction::transfer, program::{invoke, invoke_signed}}, prelude::*};
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};
use anchor_spl::metadata::{CreateMetadataAccountsV3, create_metadata_accounts_v3, mpl_token_metadata::{self, types::DataV2}};

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

    pub fn create_token(ctx: Context<CreateToken>, name: String, symbol: String, uri: String) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;

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
            authority: bonding_curve.to_account_info()
        };

        let cpi_cxt = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds);

        token::mint_to(cpi_cxt, TOKEN_TOTAL_SUPPLY)?;

        let (metadata_pda, _metadata_bump) = Pubkey::find_program_address(
            &[b"metadata", mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID.as_ref(), ctx.accounts.token_mint.key().as_ref()],  &mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID);

        require_keys_eq!(metadata_pda, ctx.accounts.metadata_account.key(), ErrorCode::InvalidProgramExecutable);

        let data = DataV2 {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            uses: None,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None
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

        let cfg = &mut ctx.accounts.global_config;

        let create_fee_ix = transfer(
            &ctx.accounts.creator.key(),
            &cfg.treasury,
            cfg.creation_fee
        );

        invoke(&create_fee_ix, &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.system_program.to_account_info()
        ])?;

        cfg.total_tokens_created = cfg
        .total_tokens_created
        .checked_add(1)
        .ok_or(ErrorCode::InvalidNumericConversion)?;

        Ok(())
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, sol_amount: u64) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let cfg = &mut ctx.accounts.global_config;
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
        let tokens_out = calculate_tokens_out(effective_sol, initial_sol_reserves, initial_token_reserves);
        
        let transaction_ix = transfer(
            &buyer.key(),
            &bonding_curve.key(),
            sol_amount.clone()
        );
        
        invoke(&transaction_ix, &[
            buyer.to_account_info(),
            bonding_curve.to_account_info(),
            ctx.accounts.system_program.to_account_info()
        ])?;


        let seeds: &[&[u8]] = &[b"bonding-curve", creator_key.as_ref(), &[bonding_curve.bump]];
        let signer_seeds = &[seeds];
        
        if fee > 0 {
            let transaction_fee_ix = transfer(&bonding_curve.key(), &cfg.treasury, fee);
            invoke_signed(&transaction_fee_ix, &[
                bonding_curve.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ], signer_seeds)?;
        }

        let transfer_token_accounts= token::Transfer {
            from: ctx.accounts.bonding_curve_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: bonding_curve.to_account_info()
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_token_accounts, 
            signer_seeds
        );

        bonding_curve.real_sol_reserves = bonding_curve.real_sol_reserves.checked_add(effective_sol).ok_or(ErrorCode::InvalidNumericConversion)?;
        bonding_curve.real_token_reserves = bonding_curve.real_token_reserves.checked_sub(tokens_out).ok_or(ErrorCode::InvalidNumericConversion)?;
        cfg.total_volume_sol = cfg.total_volume_sol.checked_add(effective_sol as u128).ok_or(ErrorCode::InvalidNumericConversion)?;

        token::transfer(cpi_ctx, tokens_out)?;

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
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

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
    pub buyer_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>
}

pub fn calculate_tokens_out(
    sol_amount: u64,
    initial_sol_reserves: u64,
    initial_token_reserves: u64,
) -> u64 {
    let k = initial_sol_reserves.checked_mul(initial_token_reserves).unwrap();
    let new_sol_reserves = initial_sol_reserves.checked_add(sol_amount).unwrap();
    let new_token_reserves = k.checked_div(new_sol_reserves).unwrap();
    let tokens_out = initial_token_reserves.checked_sub(new_token_reserves).unwrap();

    tokens_out
}

pub fn initialize_global_config(
    ctx: Context<InitializeGlobalConfig>,
    treasury: Pubkey,
) -> Result<()> {
    let cfg = &mut ctx.accounts.global_config;

    require!(treasury != Pubkey::default(), ErrorCode::InvalidProgramExecutable);

    cfg.authority = ctx.accounts.admin.key();
    cfg.treasury = treasury;

    require!(DEFAULT_BUY_FEE_BPS <= MAX_BUY_FEE_BPS, ErrorCode::InvalidProgramExecutable);
    require!(DEFAULT_SELL_FEE_BPS <= MAX_SELL_FEE_BPS, ErrorCode::InvalidProgramExecutable);
    require!(DEFAULT_CREATION_FEE <= MAX_CREATION_FEE, ErrorCode::InvalidProgramExecutable);

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

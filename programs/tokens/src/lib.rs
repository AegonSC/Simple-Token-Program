use anchor_lang::prelude::*;
use anchor_spl::associated_token::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("FyziD2vvL9P4yNjZsaHd2k4ArSShF6Qk6piUrdpfEKG1");

#[program]
pub mod tokens {

    use super::*;

    pub fn create_token_mint(_ctx: Context<CreateToken>) -> Result<()> {
        msg!("Token mint created!");
        Ok(())
    }

    pub fn initialize_state(ctx: Context<InitializeState>, max_supply: u64) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.max_supply = max_supply;
        state.total_minted = 0;
        state.mint = ctx.accounts.mint.key();
        state.admin = ctx.accounts.admin.key();
        Ok(())
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        // 1. Valido que la cantidad sea mayor a 0
        if amount == 0 {
            return err!(ErrorCode::AmountMustBeGreaterThanZero);
        }

        let state = &mut ctx.accounts.state;

        if ctx.accounts.mint.key() != state.mint {
            return err!(ErrorCode::MintMismatch);
        }

        if state.total_minted + amount > state.max_supply {
            return err!(ErrorCode::ExceedsMaxSupply);
        }

        msg!("Minting {} tokens", amount);
        //2. Creo el contexto para la CPI al token program
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(), // Mint account of the token
            to: ctx.accounts.destination_ata.to_account_info(), // Token Account Detiny
            authority: ctx.accounts.mint_authority.to_account_info(), //Sginer that it is mint_authority
        };

        let cpi_program = ctx.accounts.token_program.to_account_info(); // Token Program
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // 3. Calling the function mint_to from Token Program
        token::mint_to(cpi_ctx, amount)?;

        state.total_minted += amount;

        msg!("Tokens minted successfully");
        Ok(())
    }

    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        if amount == 0 {
            return err!(ErrorCode::AmountMustBeGreaterThanZero);
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.from.to_account_info(),
            to: ctx.accounts.to.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;

        msg!("Transferred {} tokens", amount);
        Ok(())
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        if amount == 0 {
            return err!(ErrorCode::AmountMustBeGreaterThanZero);
        }

        let cpi_accounts = Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.from.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::burn(cpi_ctx, amount)?;

        msg!("Burned {} tokens", amount);

        Ok(())
    }

}

#[derive(Accounts)]
pub struct InitializeState<'info> {
    #[account(init, payer = admin, space = 8 + 8 + 8 + 32 + 32)]
    // space: discriminator + 2 u64 + 2 pubkey
    pub state: Account<'info, TokenState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = authority // La autoridad inicial es quien paga
    )]
    pub mint: Account<'info, Mint>,

    #[account(init_if_needed, payer=authority, associated_token::mint = mint, associated_token::authority = authority)]
    pub token_account: Account<'info, TokenAccount>, // Esta es la ATA de la autoridad

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(
        mut, // it has to be mutable 'cause his supply will change
        constraint = mint.mint_authority.unwrap() == mint_authority.key() @ ErrorCode::InvalidMintAuthority
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut, // it has to be mutable because his balance will change
    )]
    pub destination_ata: Account<'info, TokenAccount>, // La cuenta (generalmente ATA) donde irán los tokens

    // The authority that has permissions
    pub mint_authority: Signer<'info>,

    #[account(mut, has_one = mint @ ErrorCode::MintMismatch)]
    pub state: Account<'info, TokenState>,

    // The program of Tokens SPL
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = from.mint == mint.key() @ ErrorCode::MintMismatch
        
    )]
    pub from: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct TokenState {
    pub max_supply: u64,
    pub total_minted: u64,
    pub mint: Pubkey,
    pub admin: Pubkey, // this works so only the admin can initialize it
}

#[error_code]
pub enum ErrorCode {
    #[msg("The amount must be greater than zero")]
    AmountMustBeGreaterThanZero,
    #[msg("Invalid Mint Authority")]
    InvalidMintAuthority,
    #[msg("Destination token account is not associated with the correct mint.")]
    MintMismatch,
    #[msg("Minting would exceed the maximum token supply.")]
    ExceedsMaxSupply,
}

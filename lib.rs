use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use std::convert::TryInto;

declare_id!("GiCazGBqZBQEJz5CipbMMZiWfZX9FRE9kXLtwQNS2Vsj");

#[program]
pub mod gwim_presale {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, 
                     token_price: u64,
                     max_tokens_per_wallet: u64,
                     total_tokens_for_sale: u64) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        presale_state.authority = ctx.accounts.authority.key();
        presale_state.vault = ctx.accounts.vault.key();
        presale_state.token_price = token_price;
        presale_state.max_tokens_per_wallet = max_tokens_per_wallet;
        presale_state.total_tokens_for_sale = total_tokens_for_sale;
        presale_state.tokens_sold = 0;
        presale_state.is_active = true;
        
        msg!("Presale initialized with token price: {}", token_price);
        msg!("Max tokens per wallet: {}", max_tokens_per_wallet);
        msg!("Total tokens for sale: {}", total_tokens_for_sale);
        
        Ok(())
    }

    pub fn update_presale_settings(ctx: Context<UpdatePresaleSettings>,
                                  token_price: Option<u64>,
                                  max_tokens_per_wallet: Option<u64>,
                                  is_active: Option<bool>) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        // Only update fields that are provided
        if let Some(price) = token_price {
            presale_state.token_price = price;
            msg!("Updated token price to: {}", price);
        }
        
        if let Some(max_tokens) = max_tokens_per_wallet {
            presale_state.max_tokens_per_wallet = max_tokens;
            msg!("Updated max tokens per wallet to: {}", max_tokens);
        }
        
        if let Some(active_state) = is_active {
            presale_state.is_active = active_state;
            msg!("Updated presale active state to: {}", active_state);
        }
        
        Ok(())
    }

    pub fn purchase_token(ctx: Context<PurchaseToken>, amount: u64) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        // Check if presale is active
        require!(presale_state.is_active, PresaleError::PresaleNotActive);
        
        // Check if there are enough tokens left for sale
        require!(
            presale_state.tokens_sold + amount <= presale_state.total_tokens_for_sale,
            PresaleError::InsufficientTokensForSale
        );
        
        // Check if buyer is not exceeding max tokens per wallet
        let buyer_token_balance = ctx.accounts.buyer_token_account.amount;
        require!(
            buyer_token_balance + amount <= presale_state.max_tokens_per_wallet,
            PresaleError::MaxTokensPerWalletExceeded
        );
        
        // Calculate SOL amount required based on token price
        let sol_amount = (amount as u128)
            .checked_mul(presale_state.token_price as u128)
            .ok_or(PresaleError::CalculationError)?
            .try_into()
            .map_err(|_| PresaleError::CalculationError)?;
        
        // First transfer SOL from buyer to vault authority
        let sol_transfer_to_vault_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.vault_authority.key(),
            sol_amount,
        );
        
        anchor_lang::solana_program::program::invoke(
            &sol_transfer_to_vault_ix,
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.vault_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        
        // Then automatically transfer SOL from vault authority to the authority wallet
        let bump = ctx.bumps.vault_authority;
        let seeds: &[&[u8]] = &[
            b"vault",
            &[bump],
        ];
        let signer = &[&seeds[..]];
        
        let sol_transfer_to_authority_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.vault_authority.key(),
            &ctx.accounts.authority.key(),
            sol_amount,
        );
        
        anchor_lang::solana_program::program::invoke_signed(
            &sol_transfer_to_authority_ix,
            &[
                ctx.accounts.vault_authority.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;
        
        // Transfer tokens from vault to buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );

        token::transfer(cpi_ctx, amount)?;
        
        // Update tokens sold
        presale_state.tokens_sold = presale_state.tokens_sold.checked_add(amount)
            .ok_or(PresaleError::CalculationError)?;

        msg!("Transferred {} tokens to buyer", amount);
        msg!("Received {} SOL from buyer", sol_amount);
        msg!("Automatically transferred {} SOL to authority wallet", sol_amount);
        msg!("Total tokens sold: {}", presale_state.tokens_sold);
        
        Ok(())
    }
    
    pub fn withdraw_sol(ctx: Context<WithdrawSol>, amount: u64) -> Result<()> {
        let bump = ctx.bumps.vault_authority;
        let seeds: &[&[u8]] = &[
        b"vault",
        &[bump],
        ];
        let signer = &[&seeds[..]];
        
        // Transfer SOL from vault authority to recipient
        let sol_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.vault_authority.key(),
            &ctx.accounts.recipient.key(),
            amount,
        );
        
        anchor_lang::solana_program::program::invoke_signed(
            &sol_transfer_ix,
            &[
                ctx.accounts.vault_authority.to_account_info(),
                ctx.accounts.recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;
        
        msg!("Withdrew {} SOL to recipient", amount);
        Ok(())
    }
    
    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>, amount: u64) -> Result<()> {
        let bump = ctx.bumps.vault_authority;
        let seeds: &[&[u8]] = &[
        b"vault",
        &[bump],
        ];
        let signer = &[&seeds[..]];
        
        // Transfer tokens from vault to recipient
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        
        token::transfer(cpi_ctx, amount)?;
        
        msg!("Withdrew {} tokens to recipient", amount);
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct PresaleState {
    pub authority: Pubkey,         // 32 bytes
    pub vault: Pubkey,             // 32 bytes
    pub token_price: u64,          // 8 bytes
    pub max_tokens_per_wallet: u64, // 8 bytes
    pub total_tokens_for_sale: u64, // 8 bytes
    pub tokens_sold: u64,          // 8 bytes
    pub is_active: bool,           // 1 byte
    // 8 bytes for anchor discriminator
    // Total: 97 bytes
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<PresaleState>(),
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    
    /// CHECK: Vault authority is a PDA
    #[account(seeds = [b"vault"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdatePresaleSettings<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"presale_state"],
        bump,
        constraint = presale_state.authority == authority.key() @ PresaleError::Unauthorized
    )]
    pub presale_state: Account<'info, PresaleState>,
}

#[derive(Accounts)]
pub struct PurchaseToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = buyer_token_account.owner == buyer.key() @ PresaleError::InvalidTokenAccount
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    /// CHECK: Vault authority is a PDA
    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    
    /// CHECK: Authority wallet that will receive the SOL
    #[account(
        mut,
        constraint = authority.key() == presale_state.authority @ PresaleError::Unauthorized
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawSol<'info> {
    #[account(
        mut,
        constraint = presale_state.authority == authority.key() @ PresaleError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    #[account(
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    /// CHECK: Vault authority is a PDA
    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    
    /// CHECK: Recipient can be any account
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    #[account(
        mut,
        constraint = presale_state.authority == authority.key() @ PresaleError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    #[account(
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    
    /// CHECK: Vault authority is a PDA
    #[account(
        seeds = [b"vault"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum PresaleError {
    #[msg("Presale is not active")]
    PresaleNotActive,
    
    #[msg("Insufficient tokens available for sale")]
    InsufficientTokensForSale,
    
    #[msg("Maximum tokens per wallet exceeded")]
    MaxTokensPerWalletExceeded,
    
    #[msg("Calculation error")]
    CalculationError,
    
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("Invalid token account")]
    InvalidTokenAccount,
}

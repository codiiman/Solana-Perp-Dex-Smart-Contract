use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;

/// Initialize a user account
/// Users must initialize their account before trading
#[derive(Accounts)]
pub struct InitializeUser<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = User::LEN,
        seeds = [b"user", user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeUser<'info> {
    pub fn execute(ctx: Context<InitializeUser>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        let clock = Clock::get()?;

        user_account.owner = ctx.accounts.user.key();
        user_account.collateral = 0;
        user_account.position_count = 0;
        user_account.order_count = 0;
        user_account.last_interaction = clock.unix_timestamp;
        user_account.bump = ctx.bumps.get("user_account").copied().unwrap_or(0);

        msg!("Initialized user account for {}", user_account.owner);
        Ok(())
    }
}

/// Deposit collateral into user account
#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, User>,

    /// User's token account (quote asset, e.g., USDC)
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Protocol's collateral vault token account
    /// CHECK: This should be a token account owned by the protocol
    #[account(mut)]
    pub collateral_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> DepositCollateral<'info> {
    pub fn execute(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, PerpDexError::InvalidOrderSize);

        let user_account = &mut ctx.accounts.user_account;
        let clock = Clock::get()?;

        // Transfer tokens from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.collateral_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Update user account
        user_account.collateral = user_account
            .collateral
            .checked_add(amount)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;
        user_account.last_interaction = clock.unix_timestamp;

        emit!(events::CollateralDeposited {
            user: user_account.owner,
            amount,
            new_balance: user_account.collateral,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Deposited {} collateral, new balance: {}",
            amount,
            user_account.collateral
        );

        Ok(())
    }
}

/// Withdraw collateral from user account
#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key() @ PerpDexError::Unauthorized
    )]
    pub user_account: Account<'info, User>,

    /// User's token account (quote asset, e.g., USDC)
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Protocol's collateral vault token account
    /// CHECK: This should be a token account owned by the protocol
    #[account(mut)]
    pub collateral_vault: Account<'info, TokenAccount>,

    /// Protocol state to check if withdrawals are allowed
    #[account(
        seeds = [b"protocol_state"],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawCollateral<'info> {
    pub fn execute(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, PerpDexError::InvalidOrderSize);

        let user_account = &mut ctx.accounts.user_account;
        let clock = Clock::get()?;

        // Check sufficient balance
        require!(
            user_account.collateral >= amount,
            PerpDexError::InsufficientCollateral
        );

        // TODO: Check that user has no open positions that would become unhealthy
        // For now, we allow withdrawal if collateral >= amount

        // Update user account first (CEI pattern)
        user_account.collateral = user_account
            .collateral
            .checked_sub(amount)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;
        user_account.last_interaction = clock.unix_timestamp;

        // Transfer tokens from vault to user
        let seeds = &[
            b"protocol_state",
            &[ctx.accounts.protocol_state.bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.collateral_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.protocol_state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        emit!(events::CollateralWithdrawn {
            user: user_account.owner,
            amount,
            new_balance: user_account.collateral,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Withdrew {} collateral, new balance: {}",
            amount,
            user_account.collateral
        );

        Ok(())
    }
}

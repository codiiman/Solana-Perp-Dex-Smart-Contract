use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;

/// Deposit fees into insurance fund
/// This is typically called automatically when fees are collected
#[derive(Accounts)]
pub struct DepositInsuranceFund<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"insurance_fund"],
        bump = insurance_fund.bump
    )]
    pub insurance_fund: Account<'info, InsuranceFund>,

    /// Payer's token account (quote asset, e.g., USDC)
    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>,

    /// Insurance fund token account
    #[account(mut)]
    pub insurance_fund_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> DepositInsuranceFund<'info> {
    pub fn execute(ctx: Context<DepositInsuranceFund>, amount: u64) -> Result<()> {
        require!(amount > 0, PerpDexError::InvalidOrderSize);

        let insurance_fund = &mut ctx.accounts.insurance_fund;
        let clock = Clock::get()?;

        // Transfer tokens to insurance fund
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_token_account.to_account_info(),
            to: ctx.accounts.insurance_fund_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Update insurance fund
        insurance_fund.balance = insurance_fund
            .balance
            .checked_add(amount)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        insurance_fund.total_fees_collected = insurance_fund
            .total_fees_collected
            .checked_add(amount)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        insurance_fund.last_update = clock.unix_timestamp;

        emit!(events::InsuranceFundDeposited {
            amount,
            new_balance: insurance_fund.balance,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Deposited {} to insurance fund, new balance: {}",
            amount,
            insurance_fund.balance
        );

        Ok(())
    }
}

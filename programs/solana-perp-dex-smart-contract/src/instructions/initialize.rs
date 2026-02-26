use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use crate::state::*;
use crate::errors::PerpDexError;

/// Initialize the protocol state
/// This must be called first to set up the protocol admin and configuration
#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = ProtocolState::LEN,
        seeds = [b"protocol_state"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    /// Insurance fund token account (quote asset, e.g., USDC)
    #[account(
        init,
        payer = admin,
        token::mint = quote_mint,
        token::authority = insurance_fund,
        seeds = [b"insurance_fund"],
        bump
    )]
    pub insurance_fund_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = admin,
        space = InsuranceFund::LEN,
        seeds = [b"insurance_fund"],
        bump
    )]
    pub insurance_fund: Account<'info, InsuranceFund>,

    pub quote_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeProtocol<'info> {
    pub fn execute(ctx: Context<InitializeProtocol>) -> Result<()> {
        let protocol_state = &mut ctx.accounts.protocol_state;
        let insurance_fund = &mut ctx.accounts.insurance_fund;
        let clock = Clock::get()?;

        // Set admin
        protocol_state.admin = ctx.accounts.admin.key();
        protocol_state.insurance_fund = ctx.accounts.insurance_fund.key();

        // Set default fees (0.1% maker, 0.2% taker)
        protocol_state.default_maker_fee_rate = 1_000; // 0.001 * PRECISION
        protocol_state.default_taker_fee_rate = 2_000; // 0.002 * PRECISION

        // Set default liquidation parameters
        protocol_state.default_liquidation_threshold = 800_000; // 0.8 * PRECISION (80%)
        protocol_state.default_liquidation_bonus_rate = 50_000; // 0.05 * PRECISION (5%)

        // Set default funding parameters
        protocol_state.default_funding_interval = 3600; // 1 hour
        protocol_state.default_max_funding_rate = 10_000; // 0.01 * PRECISION (1% per hour)
        protocol_state.default_funding_rate_sensitivity = 100_000; // 0.1 * PRECISION (10%)

        // Set oracle parameters
        protocol_state.max_oracle_staleness = 60; // 60 seconds
        protocol_state.max_oracle_deviation = 50_000; // 0.05 * PRECISION (5%)

        protocol_state.paused = false;
        protocol_state.bump = ctx.bumps.get("protocol_state").copied().unwrap_or(0);

        // Initialize insurance fund
        insurance_fund.balance = 0;
        insurance_fund.total_fees_collected = 0;
        insurance_fund.total_bad_debt_covered = 0;
        insurance_fund.last_update = clock.unix_timestamp;
        insurance_fund.bump = ctx.bumps.get("insurance_fund").copied().unwrap_or(0);

        msg!("Protocol initialized with admin: {}", protocol_state.admin);
        Ok(())
    }
}

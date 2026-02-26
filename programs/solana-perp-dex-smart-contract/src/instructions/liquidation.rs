use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;
use crate::math;

/// Liquidate an undercollateralized position
/// Anyone can call this (permissionless liquidation)
#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", position.user.as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        mut,
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, PerpMarket>,

    #[account(
        mut,
        seeds = [b"position", position.user.as_ref(), market.key().as_ref()],
        bump = position.bump,
        constraint = !position.closed @ PerpDexError::PositionNotFound
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [b"insurance_fund"],
        bump = insurance_fund.bump
    )]
    pub insurance_fund: Account<'info, InsuranceFund>,

    /// Insurance fund token account
    #[account(mut)]
    pub insurance_fund_token_account: Account<'info, TokenAccount>,

    /// Liquidator's token account (to receive reward)
    #[account(mut)]
    pub liquidator_token_account: Account<'info, TokenAccount>,

    /// Oracle account for current price
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,

    #[account(
        seeds = [b"protocol_state"],
        bump = protocol_state.bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    pub token_program: Program<'info, Token>,
}

impl<'info> LiquidatePosition<'info> {
    pub fn execute(
        ctx: Context<LiquidatePosition>,
        liquidation_size: u64,
        current_price: u64,
    ) -> Result<()> {
        require!(liquidation_size > 0, PerpDexError::InvalidOrderSize);
        require!(current_price > 0, PerpDexError::InvalidOrderPrice);

        let user_account = &ctx.accounts.user_account;
        let market = &ctx.accounts.market;
        let position = &mut ctx.accounts.position;
        let insurance_fund = &mut ctx.accounts.insurance_fund;
        let clock = Clock::get()?;

        require!(
            liquidation_size <= position.size,
            PerpDexError::LiquidationSizeExceedsMax
        );

        // Calculate unrealized PNL
        let unrealized_pnl = math::calculate_unrealized_pnl(
            position.entry_price,
            current_price,
            position.size,
            position.is_long(),
        )?;

        // Calculate margin requirement
        let margin_requirement = math::calculate_margin_requirement(
            position.size,
            position.entry_price,
            position.leverage,
        )?;

        // Calculate health factor
        let health_factor = math::calculate_health_factor(
            position.collateral,
            unrealized_pnl,
            margin_requirement,
        )?;

        // Check if position is liquidatable
        require!(
            math::is_liquidatable(health_factor, market.liquidation_threshold),
            PerpDexError::PositionNotLiquidatable
        );

        // Calculate liquidation value
        let liquidation_value = liquidation_size
            .checked_mul(current_price)
            .and_then(|x| x.checked_div(crate::math::PRECISION))
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        // Calculate liquidation bonus
        let liquidation_bonus = math::calculate_liquidation_bonus(
            liquidation_size,
            current_price,
            market.liquidation_bonus_rate,
        )?;

        // Calculate bad debt (if any)
        let position_value = if unrealized_pnl >= 0 {
            position
                .collateral
                .checked_add(unrealized_pnl as u64)
                .ok_or_else(|| PerpDexError::MathOverflow.into())?
        } else {
            position
                .collateral
                .checked_sub(unrealized_pnl.unsigned_abs())
                .ok_or_else(|| PerpDexError::MathUnderflow.into())?
        };

        let bad_debt = if liquidation_value > position_value {
            liquidation_value
                .checked_sub(position_value)
                .ok_or_else(|| PerpDexError::MathOverflow.into())?
        } else {
            0
        };

        // Check insurance fund has enough to cover bad debt and bonus
        let total_required = bad_debt
            .checked_add(liquidation_bonus)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        require!(
            insurance_fund.balance >= total_required,
            PerpDexError::InsufficientInsuranceFund
        );

        // Calculate collateral to return (proportional to liquidation_size)
        let liquidation_ratio = liquidation_size
            .checked_mul(crate::math::PRECISION)
            .and_then(|x| x.checked_div(position.size))
            .ok_or_else(|| PerpDexError::DivisionByZero.into())?;

        let collateral_to_liquidate = position
            .collateral
            .checked_mul(liquidation_ratio)
            .and_then(|x| x.checked_div(crate::math::PRECISION))
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        // Update position
        position.size = position
            .size
            .checked_sub(liquidation_size)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        position.collateral = position
            .collateral
            .checked_sub(collateral_to_liquidate)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        if position.size == 0 {
            position.closed = true;
        }

        position.last_update = clock.unix_timestamp;

        // Update insurance fund
        insurance_fund.balance = insurance_fund
            .balance
            .checked_sub(total_required)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        insurance_fund.total_bad_debt_covered = insurance_fund
            .total_bad_debt_covered
            .checked_add(bad_debt)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        insurance_fund.last_update = clock.unix_timestamp;

        // Transfer liquidation bonus to liquidator
        let seeds = &[
            b"protocol_state",
            &[ctx.accounts.protocol_state.bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.insurance_fund_token_account.to_account_info(),
            to: ctx.accounts.liquidator_token_account.to_account_info(),
            authority: ctx.accounts.protocol_state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, liquidation_bonus)?;

        emit!(events::PositionLiquidated {
            user: position.user,
            market: position.market,
            liquidator: ctx.accounts.liquidator.key(),
            liquidated_size: liquidation_size,
            liquidation_price: current_price,
            bad_debt,
            liquidator_reward: liquidation_bonus,
            timestamp: clock.unix_timestamp,
        });

        emit!(events::InsuranceFundUsed {
            amount: total_required,
            remaining_balance: insurance_fund.balance,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Liquidated position: size={}, price={}, bad_debt={}, bonus={}",
            liquidation_size,
            current_price,
            bad_debt,
            liquidation_bonus
        );

        Ok(())
    }
}

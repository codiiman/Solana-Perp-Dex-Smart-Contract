use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;
use crate::math;

/// Update funding rate for a market
/// This should be called periodically (e.g., every hour) by a keeper
#[derive(Accounts)]
pub struct UpdateFundingRate<'info> {
    /// Anyone can call this (permissionless)
    pub updater: Signer<'info>,

    #[account(
        mut,
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = !market.paused @ PerpDexError::MarketPaused
    )]
    pub market: Account<'info, PerpMarket>,

    /// Oracle account for oracle price
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,
}

impl<'info> UpdateFundingRate<'info> {
    pub fn execute(ctx: Context<UpdateFundingRate>, oracle_price: u64) -> Result<()> {
        require!(oracle_price > 0, PerpDexError::InvalidOrderPrice);

        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        // Check if enough time has passed since last update
        require!(
            clock.unix_timestamp >= market.next_funding_time,
            PerpDexError::FundingRateUpdateTooFrequent
        );

        // Calculate mark price from AMM
        let mark_price = if market.vamm_base_reserve > 0 && market.vamm_quote_reserve > 0 {
            market
                .vamm_quote_reserve
                .checked_mul(crate::math::PRECISION)
                .and_then(|x| x.checked_div(market.vamm_base_reserve))
                .ok_or_else(|| PerpDexError::DivisionByZero.into())?
        } else {
            return Err(PerpDexError::InsufficientAMMLiquidity.into());
        };

        // Calculate premium index
        let premium_index = math::calculate_premium_index(mark_price, oracle_price)?;

        // Calculate funding rate
        let funding_rate = math::calculate_funding_rate(
            premium_index,
            market.funding_rate_sensitivity,
            market.max_funding_rate,
        )?;

        // Update market state
        market.current_funding_rate = funding_rate;
        market.premium_index = premium_index;
        market.last_funding_update = clock.unix_timestamp;
        market.next_funding_time = clock.unix_timestamp + market.funding_interval as i64;

        emit!(events::FundingRateUpdated {
            market: market.key(),
            funding_rate,
            premium_index,
            next_funding_time: market.next_funding_time,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Updated funding rate: {} (premium_index: {}, mark: {}, oracle: {})",
            funding_rate,
            premium_index,
            mark_price,
            oracle_price
        );

        Ok(())
    }
}

/// Settle funding payment for a position
/// This should be called when funding is due
#[derive(Accounts)]
pub struct SettleFundingPayment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key() @ PerpDexError::Unauthorized
    )]
    pub user_account: Account<'info, User>,

    #[account(
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, PerpMarket>,

    #[account(
        mut,
        seeds = [b"position", user.key().as_ref(), market.key().as_ref()],
        bump = position.bump,
        constraint = !position.closed @ PerpDexError::PositionNotFound
    )]
    pub position: Account<'info, Position>,
}

impl<'info> SettleFundingPayment<'info> {
    pub fn execute(ctx: Context<SettleFundingPayment>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        let market = &ctx.accounts.market;
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Calculate time elapsed since last funding update
        let time_elapsed = if clock.unix_timestamp > market.last_funding_update {
            (clock.unix_timestamp - market.last_funding_update) as u64
        } else {
            0
        };

        if time_elapsed == 0 {
            return Ok(()); // No funding to settle
        }

        // Calculate funding payment
        let funding_payment = math::calculate_funding_payment(
            market.current_funding_rate,
            position.size,
            position.is_long(),
            time_elapsed,
            market.funding_interval,
        )?;

        // Update cumulative funding payment
        position.cumulative_funding_payment = position
            .cumulative_funding_payment
            .checked_add(funding_payment)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        // Deduct funding payment from collateral (if positive, user pays; if negative, user receives)
        if funding_payment > 0 {
            // User pays funding
            require!(
                user_account.collateral >= funding_payment as u64,
                PerpDexError::InsufficientCollateral
            );
            user_account.collateral = user_account
                .collateral
                .checked_sub(funding_payment as u64)
                .ok_or_else(|| PerpDexError::MathUnderflow.into())?;
        } else {
            // User receives funding
            let payment_abs = funding_payment.unsigned_abs();
            user_account.collateral = user_account
                .collateral
                .checked_add(payment_abs)
                .ok_or_else(|| PerpDexError::MathOverflow.into())?;
        }

        position.last_update = clock.unix_timestamp;
        user_account.last_interaction = clock.unix_timestamp;

        emit!(events::FundingPaymentSettled {
            user: user_account.owner,
            market: market.key(),
            payment: funding_payment,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Settled funding payment: {} for position",
            funding_payment
        );

        Ok(())
    }
}

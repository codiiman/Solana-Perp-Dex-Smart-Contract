use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;
use crate::math;

/// Open a new position from a filled order
#[derive(Accounts)]
pub struct OpenPosition<'info> {
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
        mut,
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, PerpMarket>,

    /// Oracle account for current price
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,

    #[account(
        init,
        payer = user,
        space = Position::LEN,
        seeds = [b"position", user.key().as_ref(), market.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    pub system_program: Program<'info, System>,
}

impl<'info> OpenPosition<'info> {
    pub fn execute(
        ctx: Context<OpenPosition>,
        size: u64,
        entry_price: u64,
        leverage: u8,
        side: u8, // 0 = long, 1 = short
    ) -> Result<()> {
        require!(side <= 1, PerpDexError::InvalidOrderSide);
        require!(size > 0, PerpDexError::InvalidOrderSize);
        require!(entry_price > 0, PerpDexError::InvalidOrderPrice);
        require!(
            leverage > 0 && leverage <= ctx.accounts.market.max_leverage,
            PerpDexError::PositionSizeExceedsMaxLeverage
        );

        let user_account = &mut ctx.accounts.user_account;
        let market = &ctx.accounts.market;
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Calculate margin requirement
        let margin_requirement = math::calculate_margin_requirement(size, entry_price, leverage)?;

        // Check sufficient collateral
        require!(
            user_account.collateral >= margin_requirement,
            PerpDexError::InsufficientMargin
        );

        // Deduct collateral
        user_account.collateral = user_account
            .collateral
            .checked_sub(margin_requirement)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        // Initialize position
        position.user = user_account.owner;
        position.market = market.key();
        position.side = side;
        position.size = size;
        position.entry_price = entry_price;
        position.leverage = leverage;
        position.collateral = margin_requirement;
        position.unrealized_pnl = 0;
        position.cumulative_funding_payment = 0;
        position.last_update = clock.unix_timestamp;
        position.closed = false;
        position.bump = ctx.bumps.get("position").copied().unwrap_or(0);

        // Update user position count
        user_account.position_count = user_account
            .position_count
            .checked_add(1)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;
        user_account.last_interaction = clock.unix_timestamp;

        emit!(events::PositionOpened {
            user: user_account.owner,
            market: market.key(),
            side: if side == 0 {
                events::PositionSide::Long
            } else {
                events::PositionSide::Short
            },
            size,
            entry_price,
            leverage,
            collateral: margin_requirement,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Opened {} position: size={}, price={}, leverage={}x",
            if side == 0 { "long" } else { "short" },
            size,
            entry_price,
            leverage
        );

        Ok(())
    }
}

/// Update position PNL based on current market price
#[derive(Accounts)]
pub struct UpdatePositionPnL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
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

    /// Oracle account for current price
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,
}

impl<'info> UpdatePositionPnL<'info> {
    pub fn execute(ctx: Context<UpdatePositionPnL>, current_price: u64) -> Result<()> {
        require!(current_price > 0, PerpDexError::InvalidOrderPrice);

        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Calculate unrealized PNL
        let unrealized_pnl = math::calculate_unrealized_pnl(
            position.entry_price,
            current_price,
            position.size,
            position.is_long(),
        )?;

        position.unrealized_pnl = unrealized_pnl;
        position.last_update = clock.unix_timestamp;

        msg!(
            "Updated position PNL: {} (price: {})",
            unrealized_pnl,
            current_price
        );

        Ok(())
    }
}

/// Close a position (fully or partially)
#[derive(Accounts)]
pub struct ClosePosition<'info> {
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
        mut,
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

    /// Oracle account for exit price
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,
}

impl<'info> ClosePosition<'info> {
    pub fn execute(ctx: Context<ClosePosition>, close_size: u64) -> Result<()> {
        require!(close_size > 0, PerpDexError::InvalidOrderSize);

        let user_account = &mut ctx.accounts.user_account;
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        require!(
            close_size <= position.size,
            PerpDexError::InvalidPositionSize
        );

        // Get exit price from oracle or market
        // For now, use market AMM price as exit price
        let exit_price = if ctx.accounts.market.vamm_base_reserve > 0
            && ctx.accounts.market.vamm_quote_reserve > 0
        {
            ctx.accounts.market
                .vamm_quote_reserve
                .checked_mul(crate::math::PRECISION)
                .and_then(|x| x.checked_div(ctx.accounts.market.vamm_base_reserve))
                .ok_or_else(|| PerpDexError::DivisionByZero.into())?
        } else {
            return Err(PerpDexError::InsufficientAMMLiquidity.into());
        };

        // Calculate realized PNL
        let realized_pnl = math::calculate_unrealized_pnl(
            position.entry_price,
            exit_price,
            close_size,
            position.is_long(),
        )?;

        // Calculate collateral to return (proportional to close_size)
        let collateral_ratio = close_size
            .checked_mul(crate::math::PRECISION)
            .and_then(|x| x.checked_div(position.size))
            .ok_or_else(|| PerpDexError::DivisionByZero.into())?;

        let collateral_to_return = position
            .collateral
            .checked_mul(collateral_ratio)
            .and_then(|x| x.checked_div(crate::math::PRECISION))
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        // Add PNL to collateral (can be negative)
        let final_collateral = if realized_pnl >= 0 {
            user_account
                .collateral
                .checked_add(realized_pnl as u64)
                .and_then(|x| x.checked_add(collateral_to_return))
                .ok_or_else(|| PerpDexError::MathOverflow.into())?
        } else {
            let pnl_abs = realized_pnl.unsigned_abs();
            user_account
                .collateral
                .checked_add(collateral_to_return)
                .and_then(|x| x.checked_sub(pnl_abs))
                .ok_or_else(|| PerpDexError::MathUnderflow.into())?
        };

        user_account.collateral = final_collateral;

        // Update position
        position.size = position
            .size
            .checked_sub(close_size)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        position.collateral = position
            .collateral
            .checked_sub(collateral_to_return)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())?;

        if position.size == 0 {
            position.closed = true;
        }

        position.last_update = clock.unix_timestamp;
        user_account.last_interaction = clock.unix_timestamp;

        emit!(events::PositionClosed {
            user: user_account.owner,
            market: position.market,
            size: close_size,
            exit_price,
            pnl: realized_pnl,
            timestamp: clock.unix_timestamp,
        });

        emit!(events::PnLSettled {
            user: user_account.owner,
            market: position.market,
            realized_pnl,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Closed position: size={}, exit_price={}, pnl={}",
            close_size,
            exit_price,
            realized_pnl
        );

        Ok(())
    }
}

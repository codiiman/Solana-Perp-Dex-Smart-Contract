use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::PerpDexError;
use crate::events;
use crate::math;

/// Place a new order (limit or market)
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct PlaceOrder<'info> {
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
        bump = market.bump,
        constraint = !market.paused @ PerpDexError::MarketPaused
    )]
    pub market: Account<'info, PerpMarket>,

    #[account(
        init,
        payer = user,
        space = Order::LEN,
        seeds = [b"order", user.key().as_ref(), order_id.to_le_bytes().as_ref()],
        bump
    )]
    pub order: Account<'info, Order>,

    pub system_program: Program<'info, System>,
}

impl<'info> PlaceOrder<'info> {
    pub fn execute(
        ctx: Context<PlaceOrder>,
        order_id: u64,
        side: u8, // 0 = long/buy, 1 = short/sell
        order_type: u8, // 0 = market, 1 = limit
        size: u64,
        price: u64, // 0 for market orders
    ) -> Result<()> {
        require!(side <= 1, PerpDexError::InvalidOrderSide);
        require!(order_type <= 1, PerpDexError::InvalidOrderType);
        require!(size > 0, PerpDexError::InvalidOrderSize);

        let user_account = &mut ctx.accounts.user_account;
        let market = &ctx.accounts.market;
        let order = &mut ctx.accounts.order;
        let clock = Clock::get()?;

        // Validate limit price for limit orders
        if order_type == 1 {
            // Limit order
            require!(price > 0, PerpDexError::InvalidOrderPrice);
        }

        // Initialize order
        order.user = user_account.owner;
        order.market = market.key();
        order.order_id = order_id;
        order.side = side;
        order.order_type = order_type;
        order.size = size;
        order.filled_size = 0;
        order.price = price;
        order.status = 0; // open
        order.created_at = clock.unix_timestamp;
        order.updated_at = clock.unix_timestamp;
        order.bump = ctx.bumps.get("order").copied().unwrap_or(0);

        // Update user order count
        user_account.order_count = user_account
            .order_count
            .checked_add(1)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;
        user_account.last_interaction = clock.unix_timestamp;

        emit!(events::OrderPlaced {
            user: user_account.owner,
            market: market.key(),
            order_id,
            side: if side == 0 {
                events::OrderSide::Long
            } else {
                events::OrderSide::Short
            },
            order_type: if order_type == 0 {
                events::OrderType::Market
            } else {
                events::OrderType::Limit
            },
            size,
            price,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Placed {} order: side={}, size={}, price={}",
            if order_type == 0 { "market" } else { "limit" },
            if side == 0 { "long" } else { "short" },
            size,
            price
        );

        Ok(())
    }
}

/// Cancel an open order
#[derive(Accounts)]
pub struct CancelOrder<'info> {
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
        seeds = [b"order", user.key().as_ref(), order.order_id.to_le_bytes().as_ref()],
        bump = order.bump,
        constraint = order.user == user.key() @ PerpDexError::Unauthorized,
        constraint = order.is_open() @ PerpDexError::OrderAlreadyCancelled
    )]
    pub order: Account<'info, Order>,

    pub market: Account<'info, PerpMarket>,
}

impl<'info> CancelOrder<'info> {
    pub fn execute(ctx: Context<CancelOrder>) -> Result<()> {
        let order = &mut ctx.accounts.order;
        let clock = Clock::get()?;

        require!(order.is_open(), PerpDexError::CannotCancelFilledOrder);

        let cancelled_size = order.remaining_size();

        // Mark order as cancelled
        order.status = 2; // cancelled
        order.updated_at = clock.unix_timestamp;

        emit!(events::OrderCancelled {
            user: order.user,
            market: order.market,
            order_id: order.order_id,
            cancelled_size,
            timestamp: clock.unix_timestamp,
        });

        msg!("Cancelled order {}", order.order_id);

        Ok(())
    }
}

/// Fill an order (can be partial or full)
/// This instruction fills an order using the vAMM
#[derive(Accounts)]
pub struct FillOrder<'info> {
    #[account(mut)]
    pub filler: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", order.user.as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        mut,
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = !market.paused @ PerpDexError::MarketPaused
    )]
    pub market: Account<'info, PerpMarket>,

    #[account(
        mut,
        seeds = [b"order", order.user.as_ref(), order.order_id.to_le_bytes().as_ref()],
        bump = order.bump,
        constraint = order.is_open() @ PerpDexError::OrderAlreadyFilled
    )]
    pub order: Account<'info, Order>,

    /// Oracle account for price feed
    /// CHECK: This should be a Pyth price account
    pub oracle: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> FillOrder<'info> {
    pub fn execute(ctx: Context<FillOrder>, fill_size: u64) -> Result<()> {
        require!(fill_size > 0, PerpDexError::InvalidOrderSize);

        let order = &mut ctx.accounts.order;
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        // Check order can be filled
        let remaining_size = order.remaining_size();
        require!(
            fill_size <= remaining_size,
            PerpDexError::InvalidOrderSize
        );

        // For limit orders, check price
        if order.is_limit() {
            // Get current AMM price
            let amm_price = if market.vamm_base_reserve > 0 && market.vamm_quote_reserve > 0 {
                market
                    .vamm_quote_reserve
                    .checked_mul(crate::math::PRECISION)
                    .and_then(|x| x.checked_div(market.vamm_base_reserve))
                    .ok_or_else(|| PerpDexError::DivisionByZero.into())?
            } else {
                return Err(PerpDexError::InsufficientAMMLiquidity.into());
            };

            // For long orders: fill if AMM price <= limit price
            // For short orders: fill if AMM price >= limit price
            if order.is_long() && amm_price > order.price {
                return Err(PerpDexError::InvalidOrderPrice.into());
            }
            if order.is_short() && amm_price < order.price {
                return Err(PerpDexError::InvalidOrderPrice.into());
            }
        }

        // Execute trade on vAMM
        let is_buying_base = order.is_long();
        let (output_amount, new_base_reserve, new_quote_reserve) = Self::execute_vamm_trade(
            market.vamm_base_reserve,
            market.vamm_quote_reserve,
            fill_size,
            is_buying_base,
        )?;

        // Calculate execution price
        let execution_price = if is_buying_base {
            // Buying base: output_amount is base, fill_size is quote
            fill_size
                .checked_mul(crate::math::PRECISION)
                .and_then(|x| x.checked_div(output_amount))
                .ok_or_else(|| PerpDexError::DivisionByZero.into())?
        } else {
            // Selling base: fill_size is base, output_amount is quote
            output_amount
                .checked_mul(crate::math::PRECISION)
                .and_then(|x| x.checked_div(fill_size))
                .ok_or_else(|| PerpDexError::DivisionByZero.into())?
        };

        // Update AMM reserves
        market.vamm_base_reserve = new_base_reserve;
        market.vamm_quote_reserve = new_quote_reserve;
        market.vamm_k = new_base_reserve
            .checked_mul(new_quote_reserve)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        // Calculate fees
        let fee_rate = if order.is_limit() {
            market.maker_fee_rate
        } else {
            market.taker_fee_rate
        };

        let trade_value = fill_size
            .checked_mul(execution_price)
            .and_then(|x| x.checked_div(crate::math::PRECISION))
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        let fee = math::calculate_trade_fee(fill_size, execution_price, fee_rate)?;

        // Update order
        order.filled_size = order
            .filled_size
            .checked_add(fill_size)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        if order.filled_size >= order.size {
            order.status = 1; // filled
        }

        order.updated_at = clock.unix_timestamp;

        emit!(events::OrderFilled {
            user: order.user,
            market: order.market,
            order_id: order.order_id,
            filled_size: fill_size,
            filled_price: execution_price,
            remaining_size: order.remaining_size(),
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Filled order {}: size={}, price={}, fee={}",
            order.order_id,
            fill_size,
            execution_price,
            fee
        );

        Ok(())
    }

    /// Execute a trade on the vAMM
    fn execute_vamm_trade(
        base_reserve: u64,
        quote_reserve: u64,
        input_amount: u64,
        is_buying_base: bool,
    ) -> Result<(u64, u64, u64)> {
        let output_amount = math::calculate_vamm_output(
            base_reserve,
            quote_reserve,
            input_amount,
            is_buying_base,
        )?;

        let (new_base_reserve, new_quote_reserve) = if is_buying_base {
            // Buying base: input is quote, output is base
            (
                base_reserve
                    .checked_sub(output_amount)
                    .ok_or_else(|| PerpDexError::MathUnderflow.into())?,
                quote_reserve
                    .checked_add(input_amount)
                    .ok_or_else(|| PerpDexError::MathOverflow.into())?,
            )
        } else {
            // Selling base: input is base, output is quote
            (
                base_reserve
                    .checked_add(input_amount)
                    .ok_or_else(|| PerpDexError::MathOverflow.into())?,
                quote_reserve
                    .checked_sub(output_amount)
                    .ok_or_else(|| PerpDexError::MathUnderflow.into())?,
            )
        };

        Ok((output_amount, new_base_reserve, new_quote_reserve))
    }
}

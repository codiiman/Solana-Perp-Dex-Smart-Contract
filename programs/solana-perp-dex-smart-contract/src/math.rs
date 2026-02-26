use anchor_lang::prelude::*;
use crate::errors::PerpDexError;

/// Math utilities for perpetual DEX calculations
/// All calculations use fixed-point arithmetic with 1e6 precision (1,000,000 = 1.0)

pub const PRECISION: u64 = 1_000_000;
pub const PRECISION_I64: i64 = 1_000_000;

/// Calculate unrealized PNL for a position
/// PNL = (current_price - entry_price) * size * side_multiplier
/// For long: side_multiplier = 1, for short: side_multiplier = -1
pub fn calculate_unrealized_pnl(
    entry_price: u64,
    current_price: u64,
    size: u64,
    is_long: bool,
) -> Result<i64> {
    let price_diff = if is_long {
        current_price as i64 - entry_price as i64
    } else {
        entry_price as i64 - current_price as i64
    };

    // size is in base units (e.g., 1e6 for 1 token), price is in quote units per base unit
    // PNL = price_diff * size / PRECISION
    price_diff
        .checked_mul(size as i64)
        .and_then(|x| x.checked_div(PRECISION_I64))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Calculate funding payment for a position
/// Funding payment = funding_rate * position_size * time_elapsed / funding_interval
/// Positive funding rate means longs pay shorts, negative means shorts pay longs
pub fn calculate_funding_payment(
    funding_rate: i64, // in PRECISION units (1e6 = 1.0)
    position_size: u64,
    is_long: bool,
    time_elapsed: u64, // seconds
    funding_interval: u64, // seconds (typically 3600)
) -> Result<i64> {
    // For longs: payment = funding_rate * size * time_elapsed / funding_interval
    // For shorts: payment = -funding_rate * size * time_elapsed / funding_interval
    let rate_multiplier = if is_long { 1i64 } else { -1i64 };
    
    let payment = funding_rate
        .checked_mul(rate_multiplier)
        .and_then(|x| x.checked_mul(position_size as i64))
        .and_then(|x| x.checked_mul(time_elapsed as i64))
        .and_then(|x| x.checked_div(funding_interval as i64))
        .and_then(|x| x.checked_div(PRECISION_I64))
        .ok_or_else(|| PerpDexError::MathOverflow.into())?;

    Ok(payment)
}

/// Calculate funding rate based on premium index
/// Funding rate = clamp(premium_index * funding_rate_sensitivity, -max_funding_rate, max_funding_rate)
/// Premium index = (mark_price - oracle_price) / oracle_price
pub fn calculate_funding_rate(
    premium_index: i64, // in PRECISION units
    funding_rate_sensitivity: u64, // typically 0.1 * PRECISION
    max_funding_rate: u64, // typically 0.01 * PRECISION (1% per hour)
) -> Result<i64> {
    let sensitivity = funding_rate_sensitivity as i64;
    
    let rate = premium_index
        .checked_mul(sensitivity)
        .and_then(|x| x.checked_div(PRECISION_I64))
        .ok_or_else(|| PerpDexError::MathOverflow.into())?;

    // Clamp to max_funding_rate
    let max_rate = max_funding_rate as i64;
    let min_rate = -(max_rate);
    
    Ok(rate.max(min_rate).min(max_rate))
}

/// Calculate premium index (mark price deviation from oracle)
/// Premium index = (mark_price - oracle_price) * PRECISION / oracle_price
pub fn calculate_premium_index(mark_price: u64, oracle_price: u64) -> Result<i64> {
    if oracle_price == 0 {
        return Err(PerpDexError::DivisionByZero.into());
    }

    let diff = mark_price as i64 - oracle_price as i64;
    diff.checked_mul(PRECISION_I64)
        .and_then(|x| x.checked_div(oracle_price as i64))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Calculate margin requirement for a position
/// Margin = position_size * entry_price / leverage
pub fn calculate_margin_requirement(
    position_size: u64,
    entry_price: u64,
    leverage: u8,
) -> Result<u64> {
    if leverage == 0 {
        return Err(PerpDexError::DivisionByZero.into());
    }

    position_size
        .checked_mul(entry_price)
        .and_then(|x| x.checked_div(leverage as u64))
        .and_then(|x| x.checked_div(PRECISION))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Calculate health factor for a position
/// Health = (collateral + unrealized_pnl) / margin_requirement
/// Health < 1.0 means position is undercollateralized
pub fn calculate_health_factor(
    collateral: u64,
    unrealized_pnl: i64,
    margin_requirement: u64,
) -> Result<u64> {
    if margin_requirement == 0 {
        return Err(PerpDexError::DivisionByZero.into());
    }

    let total_value = if unrealized_pnl >= 0 {
        collateral
            .checked_add(unrealized_pnl as u64)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?
    } else {
        collateral
            .checked_sub(unrealized_pnl.unsigned_abs())
            .ok_or_else(|| PerpDexError::MathOverflow.into())?
    };

    total_value
        .checked_mul(PRECISION)
        .and_then(|x| x.checked_div(margin_requirement))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Calculate liquidation threshold
/// Position is liquidatable if health_factor < liquidation_threshold (typically 0.8 = 80%)
pub fn is_liquidatable(health_factor: u64, liquidation_threshold: u64) -> bool {
    health_factor < liquidation_threshold
}

/// Calculate liquidation bonus for liquidator
/// Bonus = liquidated_size * entry_price * liquidation_bonus_rate
pub fn calculate_liquidation_bonus(
    liquidated_size: u64,
    liquidation_price: u64,
    liquidation_bonus_rate: u64, // typically 0.05 * PRECISION (5%)
) -> Result<u64> {
    liquidated_size
        .checked_mul(liquidation_price)
        .and_then(|x| x.checked_mul(liquidation_bonus_rate))
        .and_then(|x| x.checked_div(PRECISION))
        .and_then(|x| x.checked_div(PRECISION))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// vAMM constant product formula: x * y = k
/// Calculate output amount for a given input amount
/// output = (k / (x + input)) - y (for selling base)
/// output = y - (k / (x + input)) (for buying base)
pub fn calculate_vamm_output(
    reserve_x: u64, // base asset reserve
    reserve_y: u64, // quote asset reserve
    input_amount: u64,
    is_buying_base: bool, // true = buying base with quote, false = selling base for quote
) -> Result<u64> {
    if reserve_x == 0 || reserve_y == 0 {
        return Err(PerpDexError::InsufficientAMMLiquidity.into());
    }

    let k = reserve_x
        .checked_mul(reserve_y)
        .ok_or_else(|| PerpDexError::MathOverflow.into())?;

    if is_buying_base {
        // Buying base: input is quote, output is base
        // new_reserve_y = reserve_y + input_amount
        // new_reserve_x = k / new_reserve_y
        // output = reserve_x - new_reserve_x
        let new_reserve_y = reserve_y
            .checked_add(input_amount)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        let new_reserve_x = k
            .checked_div(new_reserve_y)
            .ok_or_else(|| PerpDexError::DivisionByZero.into())?;

        reserve_x
            .checked_sub(new_reserve_x)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())
    } else {
        // Selling base: input is base, output is quote
        // new_reserve_x = reserve_x + input_amount
        // new_reserve_y = k / new_reserve_x
        // output = reserve_y - new_reserve_y
        let new_reserve_x = reserve_x
            .checked_add(input_amount)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        let new_reserve_y = k
            .checked_div(new_reserve_x)
            .ok_or_else(|| PerpDexError::DivisionByZero.into())?;

        reserve_y
            .checked_sub(new_reserve_y)
            .ok_or_else(|| PerpDexError::MathUnderflow.into())
    }
}

/// Calculate price impact for a trade
/// Price impact = |output_price - initial_price| / initial_price
pub fn calculate_price_impact(
    initial_price: u64,
    output_price: u64,
) -> Result<u64> {
    if initial_price == 0 {
        return Err(PerpDexError::DivisionByZero.into());
    }

    let diff = if output_price > initial_price {
        output_price - initial_price
    } else {
        initial_price - output_price
    };

    diff.checked_mul(PRECISION)
        .and_then(|x| x.checked_div(initial_price))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Calculate fees for a trade
/// Maker fee = size * price * maker_fee_rate
/// Taker fee = size * price * taker_fee_rate
pub fn calculate_trade_fee(
    size: u64,
    price: u64,
    fee_rate: u64, // in PRECISION units (e.g., 0.001 * PRECISION = 0.1%)
) -> Result<u64> {
    size.checked_mul(price)
        .and_then(|x| x.checked_mul(fee_rate))
        .and_then(|x| x.checked_div(PRECISION))
        .and_then(|x| x.checked_div(PRECISION))
        .ok_or_else(|| PerpDexError::MathOverflow.into())
}

/// Check if oracle price is stale
pub fn is_oracle_stale(last_update_time: i64, current_time: i64, max_staleness: u64) -> bool {
    let staleness = current_time - last_update_time;
    staleness > max_staleness as i64
}

/// Check if oracle price deviation is acceptable
/// Deviation = |oracle_price - mark_price| / oracle_price
pub fn is_oracle_deviation_acceptable(
    oracle_price: u64,
    mark_price: u64,
    max_deviation: u64, // in PRECISION units (e.g., 0.05 * PRECISION = 5%)
) -> Result<bool> {
    if oracle_price == 0 {
        return Err(PerpDexError::DivisionByZero.into());
    }

    let diff = if mark_price > oracle_price {
        mark_price - oracle_price
    } else {
        oracle_price - mark_price
    };

    let deviation = diff
        .checked_mul(PRECISION)
        .and_then(|x| x.checked_div(oracle_price))
        .ok_or_else(|| PerpDexError::MathOverflow.into())?;

    Ok(deviation <= max_deviation)
}

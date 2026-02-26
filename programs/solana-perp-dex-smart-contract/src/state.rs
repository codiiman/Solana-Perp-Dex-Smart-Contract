use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

/// Global protocol state - stores configuration and admin
#[account]
#[derive(Default)]
pub struct ProtocolState {
    /// Protocol admin (can update config, pause markets, etc.)
    pub admin: Pubkey,
    /// Insurance fund account
    pub insurance_fund: Pubkey,
    /// Default maker fee rate (in PRECISION units, e.g., 0.001 * PRECISION = 0.1%)
    pub default_maker_fee_rate: u64,
    /// Default taker fee rate (in PRECISION units)
    pub default_taker_fee_rate: u64,
    /// Default liquidation threshold (in PRECISION units, e.g., 0.8 * PRECISION = 80%)
    pub default_liquidation_threshold: u64,
    /// Default liquidation bonus rate (in PRECISION units, e.g., 0.05 * PRECISION = 5%)
    pub default_liquidation_bonus_rate: u64,
    /// Default funding interval in seconds (typically 3600 = 1 hour)
    pub default_funding_interval: u64,
    /// Default max funding rate per interval (in PRECISION units, e.g., 0.01 * PRECISION = 1%)
    pub default_max_funding_rate: u64,
    /// Default funding rate sensitivity (in PRECISION units, e.g., 0.1 * PRECISION = 10%)
    pub default_funding_rate_sensitivity: u64,
    /// Max oracle staleness in seconds (typically 60)
    pub max_oracle_staleness: u64,
    /// Max oracle price deviation (in PRECISION units, e.g., 0.05 * PRECISION = 5%)
    pub max_oracle_deviation: u64,
    /// Protocol paused flag
    pub paused: bool,
    /// Bump seed for PDA
    pub bump: u8,
}

impl ProtocolState {
    pub const LEN: usize = 8 + // discriminator
        32 + // admin
        32 + // insurance_fund
        8 + // default_maker_fee_rate
        8 + // default_taker_fee_rate
        8 + // default_liquidation_threshold
        8 + // default_liquidation_bonus_rate
        8 + // default_funding_interval
        8 + // default_max_funding_rate
        8 + // default_funding_rate_sensitivity
        8 + // max_oracle_staleness
        8 + // max_oracle_deviation
        1 + // paused
        1; // bump
}

/// Perpetual market configuration
#[account]
pub struct PerpMarket {
    /// Market identifier (unique)
    pub market_id: u64,
    /// Base asset symbol (e.g., "SOL", "BTC")
    pub base_asset: String,
    /// Quote asset symbol (e.g., "USDC")
    pub quote_asset: String,
    /// Oracle price feed account (Pyth or other)
    pub oracle: Pubkey,
    /// Maximum leverage allowed (e.g., 20 = 20x)
    pub max_leverage: u8,
    /// Maker fee rate for this market (in PRECISION units)
    pub maker_fee_rate: u64,
    /// Taker fee rate for this market (in PRECISION units)
    pub taker_fee_rate: u64,
    /// Liquidation threshold (in PRECISION units)
    pub liquidation_threshold: u64,
    /// Liquidation bonus rate (in PRECISION units)
    pub liquidation_bonus_rate: u64,
    /// Funding interval in seconds
    pub funding_interval: u64,
    /// Max funding rate per interval (in PRECISION units)
    pub max_funding_rate: u64,
    /// Funding rate sensitivity (in PRECISION units)
    pub funding_rate_sensitivity: u64,
    /// Current funding rate (in PRECISION units, can be negative)
    pub current_funding_rate: i64,
    /// Premium index (mark price deviation from oracle, in PRECISION units)
    pub premium_index: i64,
    /// Last funding update timestamp
    pub last_funding_update: i64,
    /// Next funding time
    pub next_funding_time: i64,
    /// vAMM base reserve (for constant product AMM)
    pub vamm_base_reserve: u64,
    /// vAMM quote reserve (for constant product AMM)
    pub vamm_quote_reserve: u64,
    /// vAMM invariant (k = x * y)
    pub vamm_k: u64,
    /// Market paused flag
    pub paused: bool,
    /// Bump seed for PDA
    pub bump: u8,
}

impl PerpMarket {
    pub const LEN: usize = 8 + // discriminator
        8 + // market_id
        32 + // base_asset (String max length)
        32 + // quote_asset (String max length)
        32 + // oracle
        1 + // max_leverage
        8 + // maker_fee_rate
        8 + // taker_fee_rate
        8 + // liquidation_threshold
        8 + // liquidation_bonus_rate
        8 + // funding_interval
        8 + // max_funding_rate
        8 + // funding_rate_sensitivity
        8 + // current_funding_rate (i64)
        8 + // premium_index (i64)
        8 + // last_funding_update
        8 + // next_funding_time
        8 + // vamm_base_reserve
        8 + // vamm_quote_reserve
        8 + // vamm_k
        1 + // paused
        1; // bump
}

/// User account - stores user's collateral and positions
#[account]
pub struct User {
    /// User's wallet address
    pub owner: Pubkey,
    /// Total collateral deposited (in quote asset units, e.g., USDC)
    pub collateral: u64,
    /// Total number of positions
    pub position_count: u64,
    /// Total number of open orders
    pub order_count: u64,
    /// Last interaction timestamp
    pub last_interaction: i64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl User {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        8 + // collateral
        8 + // position_count
        8 + // order_count
        8 + // last_interaction
        1; // bump
}

/// User position in a perpetual market
#[account]
pub struct Position {
    /// User account this position belongs to
    pub user: Pubkey,
    /// Market this position is in
    pub market: Pubkey,
    /// Position side (0 = long, 1 = short)
    pub side: u8, // 0 = long, 1 = short
    /// Position size in base asset units
    pub size: u64,
    /// Entry price (in quote units per base unit, with PRECISION)
    pub entry_price: u64,
    /// Leverage used (e.g., 10 = 10x)
    pub leverage: u8,
    /// Collateral allocated to this position
    pub collateral: u64,
    /// Unrealized PNL (can be negative)
    pub unrealized_pnl: i64,
    /// Last funding payment (cumulative, can be negative)
    pub cumulative_funding_payment: i64,
    /// Last update timestamp
    pub last_update: i64,
    /// Position is closed flag
    pub closed: bool,
    /// Bump seed for PDA
    pub bump: u8,
}

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // user
        32 + // market
        1 + // side
        8 + // size
        8 + // entry_price
        1 + // leverage
        8 + // collateral
        8 + // unrealized_pnl (i64)
        8 + // cumulative_funding_payment (i64)
        8 + // last_update
        1 + // closed
        1; // bump

    pub fn is_long(&self) -> bool {
        self.side == 0
    }

    pub fn is_short(&self) -> bool {
        self.side == 1
    }
}

/// Order in the orderbook
#[account]
pub struct Order {
    /// User account this order belongs to
    pub user: Pubkey,
    /// Market this order is for
    pub market: Pubkey,
    /// Order ID (unique per user)
    pub order_id: u64,
    /// Order side (0 = long/buy, 1 = short/sell)
    pub side: u8, // 0 = long/buy, 1 = short/sell
    /// Order type (0 = market, 1 = limit)
    pub order_type: u8, // 0 = market, 1 = limit
    /// Order size in base asset units
    pub size: u64,
    /// Filled size in base asset units
    pub filled_size: u64,
    /// Limit price (in quote units per base unit, with PRECISION, 0 for market orders)
    pub price: u64,
    /// Order status (0 = open, 1 = filled, 2 = cancelled)
    pub status: u8, // 0 = open, 1 = filled, 2 = cancelled
    /// Creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl Order {
    pub const LEN: usize = 8 + // discriminator
        32 + // user
        32 + // market
        8 + // order_id
        1 + // side
        1 + // order_type
        8 + // size
        8 + // filled_size
        8 + // price
        1 + // status
        8 + // created_at
        8 + // updated_at
        1; // bump

    pub fn is_long(&self) -> bool {
        self.side == 0
    }

    pub fn is_short(&self) -> bool {
        self.side == 1
    }

    pub fn is_market(&self) -> bool {
        self.order_type == 0
    }

    pub fn is_limit(&self) -> bool {
        self.order_type == 1
    }

    pub fn is_open(&self) -> bool {
        self.status == 0
    }

    pub fn is_filled(&self) -> bool {
        self.status == 1
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == 2
    }

    pub fn remaining_size(&self) -> u64 {
        self.size.saturating_sub(self.filled_size)
    }
}

/// Insurance fund - accumulates fees and covers bad debt from liquidations
#[account]
pub struct InsuranceFund {
    /// Total balance in quote asset units
    pub balance: u64,
    /// Total fees collected
    pub total_fees_collected: u64,
    /// Total bad debt covered
    pub total_bad_debt_covered: u64,
    /// Last update timestamp
    pub last_update: i64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl InsuranceFund {
    pub const LEN: usize = 8 + // discriminator
        8 + // balance
        8 + // total_fees_collected
        8 + // total_bad_debt_covered
        8 + // last_update
        1; // bump
}

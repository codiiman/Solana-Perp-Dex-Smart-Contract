use anchor_lang::prelude::*;

/// Event emitted when a new perpetual market is created
#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub base_asset: String,
    pub quote_asset: String,
    pub oracle: Pubkey,
    pub max_leverage: u8,
    pub timestamp: i64,
}

/// Event emitted when a user places an order
#[event]
pub struct OrderPlaced {
    pub user: Pubkey,
    pub market: Pubkey,
    pub order_id: u64,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub size: u64,
    pub price: u64,
    pub timestamp: i64,
}

/// Event emitted when an order is filled (fully or partially)
#[event]
pub struct OrderFilled {
    pub user: Pubkey,
    pub market: Pubkey,
    pub order_id: u64,
    pub filled_size: u64,
    pub filled_price: u64,
    pub remaining_size: u64,
    pub timestamp: i64,
}

/// Event emitted when an order is cancelled
#[event]
pub struct OrderCancelled {
    pub user: Pubkey,
    pub market: Pubkey,
    pub order_id: u64,
    pub cancelled_size: u64,
    pub timestamp: i64,
}

/// Event emitted when a position is opened
#[event]
pub struct PositionOpened {
    pub user: Pubkey,
    pub market: Pubkey,
    pub side: PositionSide,
    pub size: u64,
    pub entry_price: u64,
    pub leverage: u8,
    pub collateral: u64,
    pub timestamp: i64,
}

/// Event emitted when a position is closed
#[event]
pub struct PositionClosed {
    pub user: Pubkey,
    pub market: Pubkey,
    pub size: u64,
    pub exit_price: u64,
    pub pnl: i64,
    pub timestamp: i64,
}

/// Event emitted when funding rate is updated
#[event]
pub struct FundingRateUpdated {
    pub market: Pubkey,
    pub funding_rate: i64,
    pub premium_index: i64,
    pub next_funding_time: i64,
    pub timestamp: i64,
}

/// Event emitted when funding payment is settled
#[event]
pub struct FundingPaymentSettled {
    pub user: Pubkey,
    pub market: Pubkey,
    pub payment: i64,
    pub timestamp: i64,
}

/// Event emitted when a position is liquidated
#[event]
pub struct PositionLiquidated {
    pub user: Pubkey,
    pub market: Pubkey,
    pub liquidator: Pubkey,
    pub liquidated_size: u64,
    pub liquidation_price: u64,
    pub bad_debt: u64,
    pub liquidator_reward: u64,
    pub timestamp: i64,
}

/// Event emitted when PNL is settled
#[event]
pub struct PnLSettled {
    pub user: Pubkey,
    pub market: Pubkey,
    pub realized_pnl: i64,
    pub timestamp: i64,
}

/// Event emitted when collateral is deposited
#[event]
pub struct CollateralDeposited {
    pub user: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when collateral is withdrawn
#[event]
pub struct CollateralWithdrawn {
    pub user: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when insurance fund receives funds
#[event]
pub struct InsuranceFundDeposited {
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when insurance fund is used for liquidation
#[event]
pub struct InsuranceFundUsed {
    pub amount: u64,
    pub remaining_balance: u64,
    pub timestamp: i64,
}

/// Order side enum for events
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum OrderSide {
    Long,
    Short,
}

/// Order type enum for events
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
}

/// Position side enum for events
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
}

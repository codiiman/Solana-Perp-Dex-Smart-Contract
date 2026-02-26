use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod math;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("PerpDex1111111111111111111111111111111");

/// Solana Perpetual DEX Smart Contract
/// 
/// A production-grade perpetual futures DEX inspired by Drift Protocol V2.
/// Supports:
/// - Multiple perpetual markets with configurable parameters
/// - Long/short positions with leverage
/// - Limit and market orders
/// - vAMM (virtual AMM) for price discovery
/// - Funding rate mechanism
/// - Permissionless liquidations
/// - Insurance fund for bad debt coverage
/// - Oracle price feeds (Pyth integration ready)
#[program]
pub mod solana_perp_dex_smart_contract {
    use super::*;

    /// Initialize the protocol
    /// Sets up the protocol state, admin, and insurance fund
    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        InitializeProtocol::execute(ctx)
    }

    /// Create a new perpetual market
    /// Only the protocol admin can create markets
    pub fn create_perp_market(
        ctx: Context<CreatePerpMarket>,
        market_id: u64,
        base_asset: String,
        quote_asset: String,
        max_leverage: u8,
        initial_base_reserve: u64,
        initial_quote_reserve: u64,
    ) -> Result<()> {
        CreatePerpMarket::execute(
            ctx,
            market_id,
            base_asset,
            quote_asset,
            max_leverage,
            initial_base_reserve,
            initial_quote_reserve,
        )
    }

    /// Update market configuration
    /// Only the protocol admin can update market configs
    pub fn update_market_config(
        ctx: Context<UpdateMarketConfig>,
        max_leverage: Option<u8>,
        maker_fee_rate: Option<u64>,
        taker_fee_rate: Option<u64>,
        paused: Option<bool>,
    ) -> Result<()> {
        UpdateMarketConfig::execute(ctx, max_leverage, maker_fee_rate, taker_fee_rate, paused)
    }

    /// Initialize a user account
    /// Users must initialize their account before trading
    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        InitializeUser::execute(ctx)
    }

    /// Deposit collateral into user account
    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        amount: u64,
    ) -> Result<()> {
        DepositCollateral::execute(ctx, amount)
    }

    /// Withdraw collateral from user account
    pub fn withdraw_collateral(
        ctx: Context<WithdrawCollateral>,
        amount: u64,
    ) -> Result<()> {
        WithdrawCollateral::execute(ctx, amount)
    }

    /// Place a new order (limit or market)
    pub fn place_order(
        ctx: Context<PlaceOrder>,
        order_id: u64,
        side: u8,
        order_type: u8,
        size: u64,
        price: u64,
    ) -> Result<()> {
        PlaceOrder::execute(ctx, order_id, side, order_type, size, price)
    }

    /// Cancel an open order
    pub fn cancel_order(ctx: Context<CancelOrder>) -> Result<()> {
        CancelOrder::execute(ctx)
    }

    /// Fill an order (can be partial or full)
    /// Uses vAMM for execution
    pub fn fill_order(
        ctx: Context<FillOrder>,
        fill_size: u64,
    ) -> Result<()> {
        FillOrder::execute(ctx, fill_size)
    }

    /// Open a new position from a filled order
    pub fn open_position(
        ctx: Context<OpenPosition>,
        size: u64,
        entry_price: u64,
        leverage: u8,
        side: u8,
    ) -> Result<()> {
        OpenPosition::execute(ctx, size, entry_price, leverage, side)
    }

    /// Update position PNL based on current market price
    pub fn update_position_pnl(
        ctx: Context<UpdatePositionPnL>,
        current_price: u64,
    ) -> Result<()> {
        UpdatePositionPnL::execute(ctx, current_price)
    }

    /// Close a position (fully or partially)
    pub fn close_position(
        ctx: Context<ClosePosition>,
        close_size: u64,
    ) -> Result<()> {
        ClosePosition::execute(ctx, close_size)
    }

    /// Update funding rate for a market
    /// Should be called periodically by a keeper
    pub fn update_funding_rate(
        ctx: Context<UpdateFundingRate>,
        oracle_price: u64,
    ) -> Result<()> {
        UpdateFundingRate::execute(ctx, oracle_price)
    }

    /// Settle funding payment for a position
    pub fn settle_funding_payment(
        ctx: Context<SettleFundingPayment>,
    ) -> Result<()> {
        SettleFundingPayment::execute(ctx)
    }

    /// Liquidate an undercollateralized position
    /// Permissionless - anyone can liquidate
    pub fn liquidate_position(
        ctx: Context<LiquidatePosition>,
        liquidation_size: u64,
        current_price: u64,
    ) -> Result<()> {
        LiquidatePosition::execute(ctx, liquidation_size, current_price)
    }

    /// Deposit fees into insurance fund
    pub fn deposit_insurance_fund(
        ctx: Context<DepositInsuranceFund>,
        amount: u64,
    ) -> Result<()> {
        DepositInsuranceFund::execute(ctx, amount)
    }
}

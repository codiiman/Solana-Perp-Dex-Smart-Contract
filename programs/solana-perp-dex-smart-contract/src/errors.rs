use anchor_lang::prelude::*;

/// Custom error codes for the perpetual DEX
#[error_code]
pub enum PerpDexError {
    // Market errors (0-99)
    #[msg("Market not found")]
    MarketNotFound,
    #[msg("Market already exists")]
    MarketAlreadyExists,
    #[msg("Invalid market configuration")]
    InvalidMarketConfig,
    #[msg("Market is paused")]
    MarketPaused,
    #[msg("Market oracle price is stale")]
    StaleOraclePrice,
    #[msg("Oracle price deviation too large")]
    OraclePriceDeviationTooLarge,

    // Order errors (100-199)
    #[msg("Invalid order size")]
    InvalidOrderSize,
    #[msg("Invalid order price")]
    InvalidOrderPrice,
    #[msg("Order not found")]
    OrderNotFound,
    #[msg("Order already filled")]
    OrderAlreadyFilled,
    #[msg("Order already cancelled")]
    OrderAlreadyCancelled,
    #[msg("Cannot cancel filled order")]
    CannotCancelFilledOrder,
    #[msg("Order size exceeds market limit")]
    OrderSizeExceedsLimit,
    #[msg("Invalid order side")]
    InvalidOrderSide,
    #[msg("Invalid order type")]
    InvalidOrderType,

    // Position errors (200-299)
    #[msg("Position not found")]
    PositionNotFound,
    #[msg("Insufficient margin")]
    InsufficientMargin,
    #[msg("Position size exceeds maximum leverage")]
    PositionSizeExceedsMaxLeverage,
    #[msg("Position health factor below threshold")]
    PositionUnhealthy,
    #[msg("Cannot close position with open orders")]
    CannotClosePositionWithOpenOrders,
    #[msg("Position already liquidated")]
    PositionAlreadyLiquidated,
    #[msg("Invalid position size")]
    InvalidPositionSize,

    // Funding errors (300-399)
    #[msg("Funding rate update too frequent")]
    FundingRateUpdateTooFrequent,
    #[msg("Invalid funding rate")]
    InvalidFundingRate,
    #[msg("Funding payment calculation error")]
    FundingPaymentError,

    // Liquidation errors (400-499)
    #[msg("Position is not liquidatable")]
    PositionNotLiquidatable,
    #[msg("Liquidation size exceeds maximum")]
    LiquidationSizeExceedsMax,
    #[msg("Insufficient insurance fund for liquidation")]
    InsufficientInsuranceFund,
    #[msg("Liquidation bonus exceeds limit")]
    LiquidationBonusExceedsLimit,

    // User/Account errors (500-599)
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("User account not initialized")]
    UserNotInitialized,
    #[msg("Insufficient collateral balance")]
    InsufficientCollateral,
    #[msg("Invalid collateral token")]
    InvalidCollateralToken,
    #[msg("Account overflow")]
    AccountOverflow,
    #[msg("Account underflow")]
    AccountUnderflow,

    // Math/Calculation errors (600-699)
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Math underflow")]
    MathUnderflow,
    #[msg("Invalid calculation result")]
    InvalidCalculation,

    // AMM errors (700-799)
    #[msg("Insufficient liquidity in AMM")]
    InsufficientAMMLiquidity,
    #[msg("Invalid AMM parameters")]
    InvalidAMMParameters,
    #[msg("AMM price impact too large")]
    AMMPriceImpactTooLarge,
    #[msg("AMM invariant violation")]
    AMMInvariantViolation,

    // Insurance fund errors (800-899)
    #[msg("Insurance fund insufficient")]
    InsuranceFundInsufficient,
    #[msg("Invalid insurance fund operation")]
    InvalidInsuranceFundOperation,

    // General errors (900-999)
    #[msg("Invalid account state")]
    InvalidAccountState,
    #[msg("Operation not allowed")]
    OperationNotAllowed,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("System error")]
    SystemError,
}

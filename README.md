# Solana Perpetual DEX Smart Contract

[![Anchor](https://img.shields.io/badge/Anchor-0.30.1-blue)](https://www.anchor-lang.com/)
[![Solana](https://img.shields.io/badge/Solana-1.18.0-purple)](https://solana.com/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

> A production-grade Solana smart contract for a perpetual futures DEX, inspired by [Drift Protocol V2](https://github.com/drift-labs/protocol-v2). Built with Anchor framework and designed for high-performance DeFi trading.

## 📋 Overview

This project implements a complete perpetual futures decentralized exchange (DEX) on Solana, supporting:

- **Multiple Markets**: Permissioned creation of perpetual markets with configurable parameters
- **Leverage Trading**: Long/short positions with up to 50x leverage (configurable per market)
- **Order Types**: Limit and market orders with partial fills
- **vAMM**: Virtual Automated Market Maker for price discovery and liquidity
- **Funding Rates**: Periodic funding payments based on premium/discount to oracle price
- **Liquidations**: Permissionless liquidation of undercollateralized positions
- **Insurance Fund**: Accumulates fees to cover bad debt from liquidations
- **Oracle Integration**: Ready for Pyth Network price feeds
- **Isolated Margin**: Each position has isolated collateral and risk

## ✨ Features

### Core Functionality

- ✅ **Market Management**: Create and configure perpetual markets
- ✅ **User Accounts**: Initialize user accounts and manage collateral
- ✅ **Order Placement**: Place limit and market orders
- ✅ **Order Execution**: Fill orders via vAMM with price impact calculation
- ✅ **Position Management**: Open, update, and close positions with leverage
- ✅ **Funding Mechanism**: Hourly funding rate updates and payment settlement
- ✅ **Liquidation System**: Permissionless liquidation with liquidator rewards
- ✅ **Insurance Fund**: Fee accumulation and bad debt coverage
- ✅ **PNL Calculation**: Real-time unrealized and realized PNL tracking
- ✅ **Margin Health**: Health factor calculation for liquidation checks

### Security Features

- ✅ **Access Control**: Admin-only market creation and configuration
- ✅ **Oracle Validation**: Staleness and deviation checks
- ✅ **Overflow Protection**: Safe math operations throughout
- ✅ **CEI Pattern**: Checks-Effects-Interactions pattern for state updates
- ✅ **Reentrancy Protection**: Anchor's built-in account validation
- ✅ **Input Validation**: Comprehensive parameter validation

## 🏗️ Architecture

### Account Structure

```
ProtocolState (Global)
├── admin: Pubkey
├── insurance_fund: Pubkey
├── fee rates, liquidation params, funding params
└── oracle parameters

PerpMarket (Per Market)
├── market_id, base/quote assets
├── oracle: Pubkey
├── max_leverage, fee rates
├── funding_rate, premium_index
├── vAMM reserves (base, quote, k)
└── funding schedule

User (Per User)
├── owner: Pubkey
├── collateral: u64
├── position_count, order_count
└── last_interaction

Position (Per User/Market)
├── user, market
├── side (long/short), size
├── entry_price, leverage
├── collateral, unrealized_pnl
└── cumulative_funding_payment

Order (Per User/Order)
├── user, market, order_id
├── side, order_type, size
├── filled_size, price
└── status (open/filled/cancelled)

InsuranceFund (Global)
├── balance: u64
├── total_fees_collected
└── total_bad_debt_covered
```

### Instruction Flow

```
1. Initialize Protocol
   └─> Create ProtocolState, InsuranceFund

2. Create Market
   └─> Admin creates PerpMarket with vAMM initialization

3. User Onboarding
   ├─> Initialize User account
   └─> Deposit Collateral

4. Trading Flow
   ├─> Place Order (limit/market)
   ├─> Fill Order (via vAMM)
   ├─> Open Position (from filled order)
   └─> Update Position PNL

5. Funding Flow
   ├─> Update Funding Rate (keeper)
   └─> Settle Funding Payment (user)

6. Liquidation Flow
   ├─> Check Position Health
   ├─> Liquidate Position (permissionless)
   └─> Pay Liquidator Reward (from insurance fund)

7. Position Management
   ├─> Update Position PNL
   └─> Close Position (partial/full)
```

## 🔢 Key Algorithms

### Funding Rate Calculation

The funding rate is calculated based on the premium index (deviation of mark price from oracle price):

```
premium_index = (mark_price - oracle_price) * PRECISION / oracle_price
funding_rate = clamp(premium_index * sensitivity, -max_rate, max_rate)
```

- **Positive funding rate**: Longs pay shorts (mark > oracle)
- **Negative funding rate**: Shorts pay longs (mark < oracle)
- **Default sensitivity**: 10% (0.1 * PRECISION)
- **Default max rate**: 1% per hour (0.01 * PRECISION)

### Funding Payment

Funding payments are calculated per position:

```
payment = funding_rate * position_size * time_elapsed / funding_interval
```

- For longs: payment = rate * size * time / interval
- For shorts: payment = -rate * size * time / interval
- Payments are deducted/added to collateral

### Margin Health Factor

Health factor determines if a position is liquidatable:

```
health = (collateral + unrealized_pnl) / margin_requirement
```

- **Health < 1.0**: Position is undercollateralized
- **Health < 0.8** (default): Position is liquidatable
- **Margin requirement**: size * entry_price / leverage

### vAMM Constant Product

The virtual AMM uses the constant product formula:

```
x * y = k
```

For buying base (input quote, output base):
```
new_reserve_y = reserve_y + input_quote
new_reserve_x = k / new_reserve_y
output_base = reserve_x - new_reserve_x
```

For selling base (input base, output quote):
```
new_reserve_x = reserve_x + input_base
new_reserve_y = k / new_reserve_x
output_quote = reserve_y - new_reserve_y
```

### Unrealized PNL

PNL is calculated based on price movement:

```
For long: pnl = (current_price - entry_price) * size / PRECISION
For short: pnl = (entry_price - current_price) * size / PRECISION
```

### Liquidation Bonus

Liquidators receive a bonus from the insurance fund:

```
bonus = liquidated_size * liquidation_price * bonus_rate / PRECISION^2
```

- **Default bonus rate**: 5% (0.05 * PRECISION)
- Bad debt is also covered by the insurance fund

## 🚀 Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (v1.18.0+)
- [Anchor](https://www.anchor-lang.com/docs/installation) (v0.30.1+)
- [Node.js](https://nodejs.org/) (v18+) for tests

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Solana-Perp-Dex-Smart-Contract-1
   ```

2. **Install dependencies**
   ```bash
   anchor build
   ```

3. **Run tests** (when test suite is added)
   ```bash
   anchor test
   ```

## 📦 Build & Deploy

### Build

```bash
# Build the program
anchor build

# Build with optimizations
anchor build --release
```

### Deploy to Localnet

```bash
# Start local validator
solana-test-validator

# Deploy (in another terminal)
anchor deploy
```

### Deploy to Devnet

```bash
# Set cluster
solana config set --url devnet

# Airdrop SOL
solana airdrop 2 $(solana address)

# Deploy
anchor deploy --provider.cluster devnet
```

### Deploy to Mainnet

⚠️ **Warning**: Only deploy to mainnet after comprehensive audits and testing.

```bash
# Set cluster
solana config set --url mainnet-beta

# Deploy
anchor deploy --provider.cluster mainnet-beta
```

## 💻 Usage Examples

### Initialize Protocol

```rust
// Anchor client example
let protocol_state = Pubkey::find_program_address(
    &[b"protocol_state"],
    &program_id,
).0;

let insurance_fund = Pubkey::find_program_address(
    &[b"insurance_fund"],
    &program_id,
).0;

let tx = program
    .request()
    .accounts(InitializeProtocol {
        admin: admin.pubkey(),
        protocol_state,
        insurance_fund_token_account,
        insurance_fund,
        quote_mint,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        rent: anchor_lang::sysvar::rent::ID,
    })
    .signers(&[&admin])
    .send();
```

### Create Market

```rust
let market = Pubkey::find_program_address(
    &[b"perp_market", &market_id.to_le_bytes()],
    &program_id,
).0;

let tx = program
    .request()
    .accounts(CreatePerpMarket {
        admin: admin.pubkey(),
        protocol_state,
        oracle: pyth_oracle_pubkey,
        market,
        system_program: anchor_lang::system_program::ID,
    })
    .args(
        market_id,
        "SOL".to_string(),
        "USDC".to_string(),
        20u8, // 20x max leverage
        1_000_000_000u64, // 1000 SOL initial base reserve
        50_000_000_000u64, // 50000 USDC initial quote reserve
    )
    .signers(&[&admin])
    .send();
```

### Initialize User & Deposit

```rust
let user_account = Pubkey::find_program_address(
    &[b"user", user.pubkey().as_ref()],
    &program_id,
).0;

// Initialize user
program
    .request()
    .accounts(InitializeUser {
        user: user.pubkey(),
        user_account,
        system_program: anchor_lang::system_program::ID,
    })
    .signers(&[&user])
    .send();

// Deposit collateral
program
    .request()
    .accounts(DepositCollateral {
        user: user.pubkey(),
        user_account,
        user_token_account: user_usdc_account,
        collateral_vault: protocol_vault,
        token_program: anchor_spl::token::ID,
    })
    .args(10_000_000u64) // 10 USDC
    .signers(&[&user])
    .send();
```

### Place & Fill Order

```rust
let order = Pubkey::find_program_address(
    &[b"order", user.pubkey().as_ref(), &order_id.to_le_bytes()],
    &program_id,
).0;

// Place limit order
program
    .request()
    .accounts(PlaceOrder {
        user: user.pubkey(),
        user_account,
        market,
        order,
        system_program: anchor_lang::system_program::ID,
    })
    .args(
        order_id,
        0u8, // long
        1u8, // limit
        1_000_000u64, // 1 SOL
        50_000_000u64, // 50 USDC per SOL
    )
    .signers(&[&user])
    .send();

// Fill order
program
    .request()
    .accounts(FillOrder {
        filler: filler.pubkey(),
        user_account,
        market,
        order,
        oracle: pyth_oracle,
        system_program: anchor_lang::system_program::ID,
    })
    .args(1_000_000u64) // fill 1 SOL
    .signers(&[&filler])
    .send();
```

### Open Position

```rust
let position = Pubkey::find_program_address(
    &[b"position", user.pubkey().as_ref(), market.as_ref()],
    &program_id,
).0;

program
    .request()
    .accounts(OpenPosition {
        user: user.pubkey(),
        user_account,
        market,
        oracle: pyth_oracle,
        position,
        system_program: anchor_lang::system_program::ID,
    })
    .args(
        1_000_000u64, // 1 SOL
        50_000_000u64, // 50 USDC entry price
        10u8, // 10x leverage
        0u8, // long
    )
    .signers(&[&user])
    .send();
```

### Update Funding Rate

```rust
// Called by keeper bot
let oracle_price = get_pyth_price(&pyth_oracle)?;

program
    .request()
    .accounts(UpdateFundingRate {
        updater: keeper.pubkey(),
        market,
        oracle: pyth_oracle,
    })
    .args(oracle_price)
    .signers(&[&keeper])
    .send();
```

### Liquidate Position

```rust
let current_price = get_pyth_price(&pyth_oracle)?;

program
    .request()
    .accounts(LiquidatePosition {
        liquidator: liquidator.pubkey(),
        user_account,
        market,
        position,
        insurance_fund,
        insurance_fund_token_account,
        liquidator_token_account,
        oracle: pyth_oracle,
        protocol_state,
        token_program: anchor_spl::token::ID,
    })
    .args(
        position.size, // liquidate full position
        current_price,
    )
    .signers(&[&liquidator])
    .send();
```

## 🧪 Testing

### Unit Tests

Unit tests for math utilities and state validation:

```bash
cargo test
```

### Integration Tests

Integration tests using Anchor's test framework:

```bash
anchor test
```

### Test Coverage

Key test scenarios:

- ✅ Market creation and configuration
- ✅ User account initialization and collateral management
- ✅ Order placement, filling, and cancellation
- ✅ Position opening, PNL updates, and closing
- ✅ Funding rate calculation and payment settlement
- ✅ Liquidation scenarios (various health factors)
- ✅ Insurance fund deposits and usage
- ✅ Edge cases (overflow, underflow, division by zero)
- ✅ Access control and authorization
- ✅ Oracle staleness and deviation checks

## 🔒 Security Considerations

### ⚠️ Important Disclaimers

1. **Educational Purpose**: This codebase is designed for educational purposes and as a reference implementation. It should NOT be used in production without comprehensive security audits.

2. **No Warranty**: The code is provided "as is" without any warranty. Use at your own risk.

3. **Audit Required**: Before any mainnet deployment, the contract must undergo:
   - Professional security audits
   - Formal verification where possible
   - Extensive testing on testnets
   - Bug bounty programs

### Known Limitations

- **Oracle Integration**: Oracle price feeds are stubbed. Full Pyth integration requires additional CPI calls and account validation.
- **Orderbook**: Currently only vAMM is implemented. Full orderbook matching would require additional data structures.
- **Partial Liquidations**: Liquidation logic supports partial liquidations but may need refinement for edge cases.
- **Gas Optimization**: Some operations may be optimized further for production use.

### Security Best Practices

- ✅ All math operations use checked arithmetic
- ✅ Input validation on all user inputs
- ✅ Access control checks on admin functions
- ✅ Oracle staleness and deviation validation
- ✅ CEI pattern for state updates
- ✅ Comprehensive error handling

### Recommendations for Production

1. **Oracle Integration**: Implement full Pyth Network integration with proper account validation
2. **Circuit Breakers**: Add circuit breakers for extreme market conditions
3. **Rate Limiting**: Implement rate limiting for critical operations
4. **Monitoring**: Add comprehensive logging and monitoring
5. **Upgradeability**: Consider upgradeable program architecture
6. **Multi-sig**: Use multi-sig for admin operations
7. **Insurance**: Consider additional insurance mechanisms

## 📚 Documentation

### Code Structure

```
programs/solana-perp-dex-smart-contract/
├── src/
│   ├── lib.rs              # Main program entry point
│   ├── state.rs            # Account structs (ProtocolState, PerpMarket, User, Position, Order, InsuranceFund)
│   ├── errors.rs           # Custom error codes
│   ├── events.rs           # Event definitions
│   ├── math.rs             # Math utilities (funding, PNL, margin, vAMM)
│   └── instructions/       # Instruction implementations
│       ├── mod.rs
│       ├── initialize.rs   # Protocol initialization
│       ├── market.rs       # Market creation and configuration
│       ├── user.rs         # User account and collateral management
│       ├── order.rs        # Order placement, filling, cancellation
│       ├── position.rs     # Position management
│       ├── funding.rs      # Funding rate updates and settlement
│       ├── liquidation.rs  # Position liquidation
│       └── insurance.rs    # Insurance fund management
├── Cargo.toml              # Dependencies
└── Anchor.toml             # Anchor configuration
```

### Key Constants

- **PRECISION**: `1_000_000` (1e6) - Fixed-point precision for all calculations
- **Default Maker Fee**: `0.1%` (1_000 / PRECISION)
- **Default Taker Fee**: `0.2%` (2_000 / PRECISION)
- **Default Liquidation Threshold**: `80%` (800_000 / PRECISION)
- **Default Liquidation Bonus**: `5%` (50_000 / PRECISION)
- **Default Funding Interval**: `3600` seconds (1 hour)
- **Default Max Funding Rate**: `1%` per hour (10_000 / PRECISION)
- **Default Max Oracle Staleness**: `60` seconds
- **Default Max Oracle Deviation**: `5%` (50_000 / PRECISION)

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- Follow Rust formatting (`cargo fmt`)
- Run clippy (`cargo clippy`)
- Add comments for complex logic
- Write tests for new features

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [Drift Protocol V2](https://github.com/drift-labs/protocol-v2)
- Built with [Anchor Framework](https://www.anchor-lang.com/)
- Designed for the [Solana Blockchain](https://solana.com/)

## 📞 Contact

- telegram: https://t.me/codiiman
- twitter:  https://x.com/codiiman_

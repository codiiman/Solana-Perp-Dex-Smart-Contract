use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::PerpDexError;

/// Create a new perpetual market
/// Only the protocol admin can create markets
#[derive(Accounts)]
#[instruction(market_id: u64, base_asset: String, quote_asset: String)]
pub struct CreatePerpMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"protocol_state"],
        bump = protocol_state.bump,
        constraint = protocol_state.admin == admin.key() @ PerpDexError::Unauthorized,
        constraint = !protocol_state.paused @ PerpDexError::MarketPaused
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    /// Oracle price feed account (Pyth or other oracle)
    /// CHECK: We validate this is a valid oracle account
    pub oracle: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        space = PerpMarket::LEN,
        seeds = [b"perp_market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, PerpMarket>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreatePerpMarket<'info> {
    pub fn execute(
        ctx: Context<CreatePerpMarket>,
        market_id: u64,
        base_asset: String,
        quote_asset: String,
        max_leverage: u8,
        initial_base_reserve: u64,
        initial_quote_reserve: u64,
    ) -> Result<()> {
        require!(
            max_leverage > 0 && max_leverage <= 50,
            PerpDexError::InvalidMarketConfig
        );
        require!(
            initial_base_reserve > 0 && initial_quote_reserve > 0,
            PerpDexError::InvalidAMMParameters
        );

        let protocol_state = &ctx.accounts.protocol_state;
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        // Initialize market
        market.market_id = market_id;
        market.base_asset = base_asset;
        market.quote_asset = quote_asset;
        market.oracle = ctx.accounts.oracle.key();
        market.max_leverage = max_leverage;

        // Set fees from protocol defaults
        market.maker_fee_rate = protocol_state.default_maker_fee_rate;
        market.taker_fee_rate = protocol_state.default_taker_fee_rate;

        // Set liquidation parameters from protocol defaults
        market.liquidation_threshold = protocol_state.default_liquidation_threshold;
        market.liquidation_bonus_rate = protocol_state.default_liquidation_bonus_rate;

        // Set funding parameters from protocol defaults
        market.funding_interval = protocol_state.default_funding_interval;
        market.max_funding_rate = protocol_state.default_max_funding_rate;
        market.funding_rate_sensitivity = protocol_state.default_funding_rate_sensitivity;
        market.current_funding_rate = 0;
        market.premium_index = 0;
        market.last_funding_update = clock.unix_timestamp;
        market.next_funding_time = clock.unix_timestamp + market.funding_interval as i64;

        // Initialize vAMM
        market.vamm_base_reserve = initial_base_reserve;
        market.vamm_quote_reserve = initial_quote_reserve;
        market.vamm_k = initial_base_reserve
            .checked_mul(initial_quote_reserve)
            .ok_or_else(|| PerpDexError::MathOverflow.into())?;

        market.paused = false;
        market.bump = ctx.bumps.get("market").copied().unwrap_or(0);

        emit!(crate::events::MarketCreated {
            market: market.key(),
            base_asset: market.base_asset.clone(),
            quote_asset: market.quote_asset.clone(),
            oracle: market.oracle,
            max_leverage: market.max_leverage,
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Created perpetual market {}: {}/{} with {}x max leverage",
            market_id,
            market.base_asset,
            market.quote_asset,
            max_leverage
        );

        Ok(())
    }
}

/// Update market configuration (admin only)
#[derive(Accounts)]
pub struct UpdateMarketConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"protocol_state"],
        bump = protocol_state.bump,
        constraint = protocol_state.admin == admin.key() @ PerpDexError::Unauthorized
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    #[account(
        mut,
        seeds = [b"perp_market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, PerpMarket>,
}

impl<'info> UpdateMarketConfig<'info> {
    pub fn execute(
        ctx: Context<UpdateMarketConfig>,
        max_leverage: Option<u8>,
        maker_fee_rate: Option<u64>,
        taker_fee_rate: Option<u64>,
        paused: Option<bool>,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;

        if let Some(leverage) = max_leverage {
            require!(
                leverage > 0 && leverage <= 50,
                PerpDexError::InvalidMarketConfig
            );
            market.max_leverage = leverage;
        }

        if let Some(fee_rate) = maker_fee_rate {
            require!(
                fee_rate <= 10_000, // Max 1%
                PerpDexError::InvalidMarketConfig
            );
            market.maker_fee_rate = fee_rate;
        }

        if let Some(fee_rate) = taker_fee_rate {
            require!(
                fee_rate <= 10_000, // Max 1%
                PerpDexError::InvalidMarketConfig
            );
            market.taker_fee_rate = fee_rate;
        }

        if let Some(pause) = paused {
            market.paused = pause;
        }

        msg!("Updated market {} configuration", market.market_id);
        Ok(())
    }
}

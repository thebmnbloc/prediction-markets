//! Foundation crate for the prediction markets quant trading system.
//!
//! Provides core domain models, exchange and strategy traits, unified error types,
//! and centralized configuration management.

pub mod config;
pub mod errors;
pub mod traits;
pub mod types;

// Re-export primary types for ergonomic usage across crates
pub use config::{
    AppConfig, BundleArbConfig, CrossArbConfig, GeneralConfig, KalshiConfig, MarketMakerConfig,
    PolymarketConfig, RiskConfig, ServerConfig, StrategiesConfig,
};
pub use errors::PmError;
pub use traits::{ExchangeAdapter, RiskCheck, Strategy};
pub use types::{
    ArbLeg, ArbitrageOpportunity, Balance, BookUpdate, Category, Exchange, Fill, Market,
    MarketFilter, MarketId, MarketStatus, NewOrder, OrderBook, OrderBookLevel, OrderConfirmation,
    OrderId, OrderStatus, OrderType, Outcome, Position, QuoteUpdate, Side, Signal,
};

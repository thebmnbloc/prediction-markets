//! Core async traits defining exchange adapters, trading strategies, and risk controls.

use crate::errors::PmError;
use crate::types::{
    Balance, BookUpdate, Exchange, Fill, Market, MarketFilter, NewOrder, OrderBook,
    OrderConfirmation, Position, Signal,
};
use async_trait::async_trait;

/// Abstraction for interacting with a prediction market exchange (Kalshi, Polymarket).
///
/// Implementors provide normalized access to market data, trading operations,
/// position queries, and account balances across different exchange protocols.
#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    /// Return the exchange identifier handled by this adapter.
    fn exchange(&self) -> Exchange;

    /// Retrieve markets matching the given filter criteria.
    async fn get_markets(&self, filter: &MarketFilter) -> Result<Vec<Market>, PmError>;

    /// Retrieve full details for a single market by identifier.
    async fn get_market(&self, market_id: &str) -> Result<Market, PmError>;

    /// Fetch the current order book snapshot for a market.
    async fn get_orderbook(&self, market_id: &str) -> Result<OrderBook, PmError>;

    /// Submit a new order to the exchange.
    async fn place_order(&self, order: &NewOrder) -> Result<OrderConfirmation, PmError>;

    /// Cancel an active order by order identifier.
    async fn cancel_order(&self, order_id: &str) -> Result<(), PmError>;

    /// Retrieve all open positions held on this exchange.
    async fn get_positions(&self) -> Result<Vec<Position>, PmError>;

    /// Query the current account cash balance.
    async fn get_balance(&self) -> Result<Balance, PmError>;
}

/// Quantitative trading strategy interface.
///
/// Strategies consume real-time order book updates and trade execution fills,
/// maintaining internal state and emitting trading signals (quotes, arbitrage orders, cancels).
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Strategy name for logging, telemetry, and configuration lookup.
    fn name(&self) -> &str;

    /// Whether this strategy is currently active and eligible to emit trade signals.
    fn is_enabled(&self) -> bool;

    /// Invoked whenever an order book update or snapshot arrives.
    async fn on_book_update(&mut self, update: &BookUpdate) -> Vec<Signal>;

    /// Invoked when a fill notification is received for an order placed by the system.
    async fn on_fill(&mut self, fill: &Fill);

    /// Strategy initialization hook (e.g. warming up caches, loading historical references).
    async fn on_start(&mut self) -> Result<(), PmError>;

    /// Strategy teardown hook (e.g. cancelling active quotes, flushing metrics).
    async fn on_stop(&mut self) -> Result<(), PmError>;
}

/// Pre-trade and real-time risk control validator.
///
/// Evaluates orders and strategy signals before execution to enforce position limits,
/// loss cutoffs, and regulatory / balance safety constraints.
#[async_trait]
pub trait RiskCheck: Send + Sync {
    /// Validate an outbound order before submitting to the exchange adapter.
    async fn check_order(&self, order: &NewOrder) -> Result<(), PmError>;

    /// Validate a strategy signal before generating execution orders.
    async fn check_signal(&self, signal: &Signal) -> Result<(), PmError>;
}

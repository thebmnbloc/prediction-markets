//! Domain types for prediction markets trading.
//!
//! Includes identifiers, market definitions, order book representation,
//! order parameters, fills, positions, balances, and strategy signals.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Supported prediction market exchanges.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    #[serde(alias = "Kalshi", alias = "KALSHI")]
    Kalshi,
    #[serde(alias = "Polymarket", alias = "POLYMARKET")]
    Polymarket,
}

/// Unique identifier for a market within an exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId {
    /// The exchange hosting this market.
    pub exchange: Exchange,
    /// Kalshi ticker (e.g. "PRES-2024") or Polymarket condition ID / token ID.
    pub id: String,
}

impl MarketId {
    /// Construct a new `MarketId`.
    pub fn new(exchange: Exchange, id: impl Into<String>) -> Self {
        Self {
            exchange,
            id: id.into(),
        }
    }
}

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.exchange, self.id)
    }
}

/// Order identifier representing either a client-assigned or exchange-assigned ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(untagged)]
pub enum OrderId {
    /// Assigned by the exchange upon order acceptance.
    Exchange(String),
    /// Assigned by the client/strategy before submission.
    Client(String),
}

impl OrderId {
    /// Return the string slice for this order ID.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Exchange(s) | Self::Client(s) => s.as_str(),
        }
    }
}

impl From<String> for OrderId {
    fn from(s: String) -> Self {
        OrderId::Exchange(s)
    }
}

impl From<&str> for OrderId {
    fn from(s: &str) -> Self {
        OrderId::Exchange(s.to_string())
    }
}

impl AsRef<str> for OrderId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Order trade side.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[serde(alias = "Buy", alias = "BUY")]
    Buy,
    #[serde(alias = "Sell", alias = "SELL")]
    Sell,
}

impl Side {
    /// Return the opposite side of the trade.
    pub fn opposite(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

/// Status of a prediction market contract.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum MarketStatus {
    #[serde(alias = "Open", alias = "OPEN", alias = "active", alias = "Active")]
    Open,
    #[serde(alias = "Closed", alias = "CLOSED")]
    Closed,
    #[serde(
        alias = "Settled",
        alias = "SETTLED",
        alias = "resolved",
        alias = "Resolved"
    )]
    Settled,
}

/// Order type specification.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    #[serde(alias = "Limit", alias = "LIMIT")]
    Limit,
    #[serde(alias = "Market", alias = "MARKET")]
    Market,
}

/// Lifecycle status of an order.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    #[serde(alias = "Open", alias = "OPEN")]
    Open,
    #[serde(alias = "Filled", alias = "FILLED")]
    Filled,
    #[serde(
        alias = "PartiallyFilled",
        alias = "partially_filled",
        alias = "PARTIALLY_FILLED"
    )]
    PartiallyFilled,
    #[serde(
        alias = "Cancelled",
        alias = "canceled",
        alias = "CANCELLED",
        alias = "CANCELED"
    )]
    Cancelled,
    #[serde(
        alias = "Failed",
        alias = "FAILED",
        alias = "rejected",
        alias = "REJECTED"
    )]
    Failed,
}

impl OrderStatus {
    /// Returns true if the order is still active and open for fills.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::PartiallyFilled)
    }

    /// Returns true if the order has reached a final terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Failed)
    }
}

/// Category of the underlying prediction event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    #[serde(alias = "Politics", alias = "POLITICS")]
    Politics,
    #[serde(alias = "Economics", alias = "ECONOMICS")]
    Economics,
    #[serde(alias = "Crypto", alias = "CRYPTO")]
    Crypto,
    #[serde(alias = "Sports", alias = "SPORTS")]
    Sports,
    #[serde(alias = "Science", alias = "SCIENCE")]
    Science,
    #[serde(alias = "Weather", alias = "WEATHER")]
    Weather,
    Other(String),
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Politics => write!(f, "politics"),
            Self::Economics => write!(f, "economics"),
            Self::Crypto => write!(f, "crypto"),
            Self::Sports => write!(f, "sports"),
            Self::Science => write!(f, "science"),
            Self::Weather => write!(f, "weather"),
            Self::Other(other) => write!(f, "{}", other),
        }
    }
}

/// Discrete outcome within a prediction market.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Outcome identifier (token id or outcome index).
    pub id: String,
    /// Outcome display title (e.g. "Yes", "No", or candidate name).
    pub title: String,
    /// Price for Yes contract (normalized 0.00 to 1.00).
    pub yes_price: Decimal,
    /// Price for No contract (normalized 0.00 to 1.00).
    pub no_price: Decimal,
}

/// Market contract details and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Market {
    /// Unique market identifier.
    pub id: MarketId,
    /// Human-readable market title / question.
    pub title: String,
    /// Event category.
    pub category: Category,
    /// Available outcomes for this market.
    pub outcomes: Vec<Outcome>,
    /// Current market trading status.
    pub status: MarketStatus,
    /// Contract resolution or expiration time, if set.
    pub expiration: Option<DateTime<Utc>>,
    /// Total trading volume in base currency units.
    pub volume: Decimal,
    /// Last update timestamp from the exchange.
    pub last_updated: DateTime<Utc>,
}

/// Single price level in an order book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookLevel {
    /// Price at this level (normalized 0.00 to 1.00 or exchange cents).
    pub price: Decimal,
    /// Aggregated quantity available at this price level.
    pub quantity: Decimal,
}

impl OrderBookLevel {
    /// Construct a new price level.
    pub fn new(price: Decimal, quantity: Decimal) -> Self {
        Self { price, quantity }
    }
}

/// L2 Order book representation for a single market.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBook {
    /// Target market identifier.
    pub market_id: MarketId,
    /// Bid orders (buys).
    pub bids: Vec<OrderBookLevel>,
    /// Ask orders (sells).
    pub asks: Vec<OrderBookLevel>,
    /// Timestamp of this book snapshot.
    pub timestamp: DateTime<Utc>,
}

impl OrderBook {
    /// Construct a new empty orderbook.
    pub fn new(market_id: MarketId, timestamp: DateTime<Utc>) -> Self {
        Self {
            market_id,
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp,
        }
    }

    /// Return the best (highest) bid level in the order book, if any.
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.iter().max_by(|a, b| a.price.cmp(&b.price))
    }

    /// Return the best (lowest) ask level in the order book, if any.
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.iter().min_by(|a, b| a.price.cmp(&b.price))
    }

    /// Calculate the mid price between the best bid and best ask.
    pub fn mid_price(&self) -> Option<Decimal> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;
        Some((bid.price + ask.price) / Decimal::from(2))
    }

    /// Calculate the bid-ask spread (best ask minus best bid).
    pub fn spread(&self) -> Option<Decimal> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;
        Some(ask.price - bid.price)
    }

    /// Sort bids descending (highest first) and asks ascending (lowest first).
    pub fn sort_levels(&mut self) {
        self.bids.sort_by(|a, b| b.price.cmp(&a.price));
        self.asks.sort_by(|a, b| a.price.cmp(&b.price));
    }
}

/// Order book snapshot or delta update message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookUpdate {
    /// Market identifier for the update.
    pub market_id: MarketId,
    /// Latest book state.
    pub book: OrderBook,
}

/// Parameters for submitting a new order to an exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewOrder {
    /// Market where the order is placed.
    pub market_id: MarketId,
    /// Buy or Sell side.
    pub side: Side,
    /// Limit or Market order.
    pub order_type: OrderType,
    /// Limit price (required for Limit orders).
    pub price: Option<Decimal>,
    /// Number of contracts or shares.
    pub quantity: Decimal,
    /// Optional client-assigned tracking identifier.
    pub client_order_id: Option<String>,
}

impl NewOrder {
    /// Create a limit order.
    pub fn limit(market_id: MarketId, side: Side, price: Decimal, quantity: Decimal) -> Self {
        Self {
            market_id,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity,
            client_order_id: None,
        }
    }

    /// Create a market order.
    pub fn market(market_id: MarketId, side: Side, quantity: Decimal) -> Self {
        Self {
            market_id,
            side,
            order_type: OrderType::Market,
            price: None,
            quantity,
            client_order_id: None,
        }
    }

    /// Attach a client order ID.
    pub fn with_client_order_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_order_id = Some(client_id.into());
        self
    }
}

/// Acknowledgement from exchange after submitting or updating an order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderConfirmation {
    /// Exchange-assigned order identifier.
    pub order_id: String,
    /// Exchange handling this order.
    pub exchange: Exchange,
    /// Target market identifier.
    pub market_id: MarketId,
    /// Current status of the order.
    pub status: OrderStatus,
    /// Total quantity filled so far.
    pub filled_quantity: Decimal,
    /// Average fill price for executed quantity, if any.
    pub avg_fill_price: Option<Decimal>,
    /// Confirmation timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Trade execution / fill notification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    /// Order identifier associated with this fill.
    pub order_id: String,
    /// Exchange where the execution occurred.
    pub exchange: Exchange,
    /// Target market identifier.
    pub market_id: MarketId,
    /// Trade side.
    pub side: Side,
    /// Execution price per contract.
    pub price: Decimal,
    /// Executed quantity.
    pub quantity: Decimal,
    /// Timestamp of fill execution.
    pub timestamp: DateTime<Utc>,
}

/// Open inventory position in a prediction market contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Target market identifier.
    pub market_id: MarketId,
    /// Side held (Buy = Long / Yes, Sell = Short / No).
    pub side: Side,
    /// Number of contracts currently held.
    pub quantity: Decimal,
    /// Volume-weighted average entry price.
    pub avg_entry_price: Decimal,
    /// Unrealized profit or loss based on current mark price.
    pub unrealized_pnl: Decimal,
    /// Realized profit or loss from closed trades.
    pub realized_pnl: Decimal,
}

/// Account balance snapshot on an exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    /// Exchange where balance is held.
    pub exchange: Exchange,
    /// Funds available for placing new orders.
    pub available: Decimal,
    /// Funds locked in open orders / margin.
    pub reserved: Decimal,
    /// Total account equity (available + reserved).
    pub total: Decimal,
}

/// Trading signal emitted by quantitative strategies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Signal {
    /// Detected cross-exchange or bundle arbitrage opportunity.
    Arbitrage(ArbitrageOpportunity),
    /// Two-sided quote update from a market maker.
    Quote(QuoteUpdate),
    /// Order cancellation request (contains order_id).
    Cancel(String),
}

/// Details for an arbitrage opportunity between two market legs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Unique identifier for this detected opportunity.
    pub id: Uuid,
    /// Buy leg details.
    pub buy_leg: ArbLeg,
    /// Sell leg details.
    pub sell_leg: ArbLeg,
    /// Gross price edge before trading fees.
    pub gross_edge: Decimal,
    /// Expected net profit edge after estimated exchange fees.
    pub net_edge: Decimal,
    /// Total estimated fees across both legs.
    pub estimated_fees: Decimal,
    /// Timestamp when opportunity was detected.
    pub detected_at: DateTime<Utc>,
}

/// A single leg of an arbitrage trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArbLeg {
    /// Target exchange for this leg.
    pub exchange: Exchange,
    /// Market identifier.
    pub market_id: MarketId,
    /// Order side (Buy or Sell).
    pub side: Side,
    /// Target price.
    pub price: Decimal,
    /// Execution quantity.
    pub quantity: Decimal,
}

/// Market maker quote instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteUpdate {
    /// Target market identifier.
    pub market_id: MarketId,
    /// Price for buy quote.
    pub bid_price: Decimal,
    /// Price for sell quote.
    pub ask_price: Decimal,
    /// Size for buy quote.
    pub bid_quantity: Decimal,
    /// Size for sell quote.
    pub ask_quantity: Decimal,
}

/// Search and filtering criteria for market queries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MarketFilter {
    /// Filter by market status.
    pub status: Option<MarketStatus>,
    /// Filter by event category.
    pub category: Option<Category>,
    /// Substring search query for market title / question.
    pub search: Option<String>,
    /// Maximum number of markets to return.
    pub limit: Option<u32>,
    /// Pagination cursor from previous page.
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_exchange_parsing_and_display() {
        assert_eq!(Exchange::Kalshi.to_string(), "kalshi");
        assert_eq!(Exchange::Polymarket.to_string(), "polymarket");

        assert_eq!(Exchange::from_str("kalshi").unwrap(), Exchange::Kalshi);
        assert_eq!(Exchange::from_str("Kalshi").unwrap(), Exchange::Kalshi);
        assert_eq!(Exchange::from_str("KALSHI").unwrap(), Exchange::Kalshi);
        assert_eq!(
            Exchange::from_str("polymarket").unwrap(),
            Exchange::Polymarket
        );
        assert_eq!(
            Exchange::from_str("Polymarket").unwrap(),
            Exchange::Polymarket
        );
    }

    #[test]
    fn test_orderbook_metrics() {
        let market_id = MarketId::new(Exchange::Kalshi, "PRES-2024");
        let now = Utc::now();
        let mut book = OrderBook::new(market_id, now);

        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
        assert!(book.mid_price().is_none());
        assert!(book.spread().is_none());

        book.bids.push(OrderBookLevel::new(dec!(0.48), dec!(100)));
        book.bids.push(OrderBookLevel::new(dec!(0.49), dec!(200)));
        book.bids.push(OrderBookLevel::new(dec!(0.47), dec!(50)));

        book.asks.push(OrderBookLevel::new(dec!(0.52), dec!(300)));
        book.asks.push(OrderBookLevel::new(dec!(0.51), dec!(150)));
        book.asks.push(OrderBookLevel::new(dec!(0.55), dec!(400)));

        let best_bid = book.best_bid().expect("should have best bid");
        assert_eq!(best_bid.price, dec!(0.49));
        assert_eq!(best_bid.quantity, dec!(200));

        let best_ask = book.best_ask().expect("should have best ask");
        assert_eq!(best_ask.price, dec!(0.51));
        assert_eq!(best_ask.quantity, dec!(150));

        assert_eq!(book.mid_price(), Some(dec!(0.50)));
        assert_eq!(book.spread(), Some(dec!(0.02)));

        // Test sorting
        book.sort_levels();
        assert_eq!(book.bids[0].price, dec!(0.49));
        assert_eq!(book.asks[0].price, dec!(0.51));
    }

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Buy.opposite(), Side::Sell);
        assert_eq!(Side::Sell.opposite(), Side::Buy);
    }

    #[test]
    fn test_market_id_display() {
        let id = MarketId::new(Exchange::Kalshi, "FED-RATE-CUT");
        assert_eq!(id.to_string(), "kalshi:FED-RATE-CUT");
    }

    #[test]
    fn test_order_id_serde_roundtrip() {
        let exchange_id = OrderId::Exchange("ord-123".into());
        let json = serde_json::to_string(&exchange_id).unwrap();
        assert_eq!(json, "\"ord-123\"");

        let deserialized: OrderId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "ord-123");
    }

    #[test]
    fn test_category_serde() {
        let cat = Category::Politics;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"politics\"");

        let deserialized: Category = serde_json::from_str("\"politics\"").unwrap();
        assert_eq!(deserialized, Category::Politics);

        let res: Result<Category, _> = serde_json::from_str("\"entertainment\"");
        assert!(res.is_err(), "unknown category should fail to deserialize");
    }

    #[test]
    fn test_signal_serialization() {
        let arb = ArbitrageOpportunity {
            id: Uuid::new_v4(),
            buy_leg: ArbLeg {
                exchange: Exchange::Kalshi,
                market_id: MarketId::new(Exchange::Kalshi, "MKT-1"),
                side: Side::Buy,
                price: dec!(0.45),
                quantity: dec!(100),
            },
            sell_leg: ArbLeg {
                exchange: Exchange::Polymarket,
                market_id: MarketId::new(Exchange::Polymarket, "0x123"),
                side: Side::Sell,
                price: dec!(0.49),
                quantity: dec!(100),
            },
            gross_edge: dec!(0.04),
            net_edge: dec!(0.035),
            estimated_fees: dec!(0.005),
            detected_at: Utc::now(),
        };

        let signal = Signal::Arbitrage(arb);
        let serialized = serde_json::to_string(&signal).unwrap();
        assert!(serialized.contains("Arbitrage"));
    }
}

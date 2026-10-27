use async_trait::async_trait;
use dashmap::DashMap;
use pm_core::errors::PmError;
use pm_core::traits::Strategy;
use pm_core::types::{BookUpdate, Exchange, Fill, Signal};
use pm_core::CrossArbConfig;

pub struct CrossArbStrategy {
    pub config: CrossArbConfig,
    pub enabled: bool,
    books: DashMap<(Exchange, String), BookUpdate>,
}

impl CrossArbStrategy {
    pub fn new(config: CrossArbConfig) -> Self {
        let enabled = config.enabled;
        Self {
            config,
            enabled,
            books: DashMap::new(),
        }
    }
}

#[async_trait]
impl Strategy for CrossArbStrategy {
    fn name(&self) -> &str {
        "cross_arb"
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    async fn on_book_update(&mut self, update: &BookUpdate) -> Vec<Signal> {
        let key = (update.market_id.exchange, update.market_id.id.clone());
        self.books.insert(key, update.clone());
        // Look for counterpart on the other exchange with same market id.
        let other_ex = match update.market_id.exchange {
            Exchange::Kalshi => Exchange::Polymarket,
            Exchange::Polymarket => Exchange::Kalshi,
        };
        let other_key = (other_ex, update.market_id.id.clone());
        if let Some(other) = self.books.get(&other_key) {
            let buy_book = &update.book;
            let sell_book = &other.book;
            if let (Some(bid), Some(ask)) = (buy_book.best_bid(), sell_book.best_ask()) {
                let edge = ask.price - bid.price; // placeholder sign
                let min_edge = rust_decimal::Decimal::new(self.config.min_edge_cents as i64, 2);
                if edge >= min_edge {
                    // Emit arbitrage signal
                    let opp = pm_core::types::ArbitrageOpportunity {
                        id: uuid::Uuid::new_v4(),
                        buy_leg: pm_core::types::ArbLeg {
                            exchange: update.market_id.exchange,
                            market_id: update.market_id.clone(),
                            side: pm_core::types::Side::Buy,
                            price: bid.price,
                            quantity: bid.quantity,
                        },
                        sell_leg: pm_core::types::ArbLeg {
                            exchange: other_ex,
                            market_id: other.market_id.clone(),
                            side: pm_core::types::Side::Sell,
                            price: ask.price,
                            quantity: ask.quantity,
                        },
                        gross_edge: edge,
                        net_edge: edge,
                        estimated_fees: rust_decimal::Decimal::ZERO,
                        detected_at: chrono::Utc::now(),
                    };
                    return vec![Signal::Arbitrage(opp)];
                }
            }
        }
        vec![]
    }
    async fn on_fill(&mut self, _fill: &Fill) {}
    async fn on_start(&mut self) -> Result<(), PmError> {
        Ok(())
    }
    async fn on_stop(&mut self) -> Result<(), PmError> {
        Ok(())
    }
}

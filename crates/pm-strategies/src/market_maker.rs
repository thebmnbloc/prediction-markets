use async_trait::async_trait;
use pm_core::errors::PmError;
use pm_core::traits::Strategy;
use pm_core::types::{BookUpdate, Fill, QuoteUpdate, Signal};
use pm_core::MarketMakerConfig;
use rust_decimal::Decimal;

pub struct MarketMakerStrategy {
    pub config: MarketMakerConfig,
    pub enabled: bool,
}

impl MarketMakerStrategy {
    pub fn new(config: MarketMakerConfig) -> Self {
        let enabled = config.enabled;
        Self { config, enabled }
    }
}

#[async_trait]
impl Strategy for MarketMakerStrategy {
    fn name(&self) -> &str {
        "market_maker"
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    async fn on_book_update(&mut self, update: &BookUpdate) -> Vec<Signal> {
        if !self.enabled {
            return vec![];
        }
        let book = &update.book;
        let mid = match book.mid_price() {
            Some(m) => m,
            None => return vec![],
        };
        let half_spread = Decimal::from(self.config.spread_width_cents) / Decimal::from(200); // cents -> dollars
        let bid = mid - half_spread;
        let ask = mid + half_spread;
        let size = Decimal::from(self.config.max_position);
        let quote = QuoteUpdate {
            market_id: update.market_id.clone(),
            bid_price: bid,
            ask_price: ask,
            bid_quantity: size,
            ask_quantity: size,
        };
        vec![Signal::Quote(quote)]
    }
    async fn on_fill(&mut self, _fill: &Fill) {}
    async fn on_start(&mut self) -> Result<(), PmError> {
        Ok(())
    }
    async fn on_stop(&mut self) -> Result<(), PmError> {
        Ok(())
    }
}

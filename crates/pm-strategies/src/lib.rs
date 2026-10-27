//! Trading strategies for prediction markets.

use dashmap::DashMap;
use pm_core::types::{BookUpdate, Exchange};

pub mod bundle_arb;
pub mod cross_arb;
pub mod market_maker;

pub use bundle_arb::BundleArbStrategy;
pub use cross_arb::CrossArbStrategy;
pub use market_maker::MarketMakerStrategy;

/// Shared book cache keyed by (exchange, market id string).
#[derive(Default)]
pub struct BookCache {
    pub books: DashMap<(Exchange, String), BookUpdate>,
}

impl BookCache {
    pub fn new() -> Self {
        Self::default()
    }
}

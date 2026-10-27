use async_trait::async_trait;
use pm_core::errors::PmError;
use pm_core::traits::Strategy;
use pm_core::types::{BookUpdate, Fill, Signal};
use pm_core::BundleArbConfig;

pub struct BundleArbStrategy {
    pub config: BundleArbConfig,
    pub enabled: bool,
}

impl BundleArbStrategy {
    pub fn new(config: BundleArbConfig) -> Self {
        let enabled = config.enabled;
        Self { config, enabled }
    }
}

#[async_trait]
impl Strategy for BundleArbStrategy {
    fn name(&self) -> &str {
        "bundle_arb"
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    async fn on_book_update(&mut self, _update: &BookUpdate) -> Vec<Signal> {
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

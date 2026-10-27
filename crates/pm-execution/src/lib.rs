use pm_core::errors::PmError;
use pm_core::types::{Fill, NewOrder, Signal};
use pm_core::RiskCheck;
use pm_core::RiskConfig;
use pm_risk::RiskEngine;
use std::sync::Arc;

pub struct ExecutionEngine {
    pub dry_run: bool,
    pub risk: Arc<RiskEngine>,
    pub fills: Vec<Fill>,
}

impl ExecutionEngine {
    pub fn new(dry_run: bool, risk_config: RiskConfig) -> Self {
        Self {
            dry_run,
            risk: Arc::new(RiskEngine::new(risk_config)),
            fills: Vec::new(),
        }
    }

    pub async fn route_signal(&mut self, signal: &Signal) -> Result<(), PmError> {
        if self.risk.is_killed() {
            return Err(PmError::kill_switch("trading halted"));
        }
        self.risk.check_signal(signal).await?;
        if self.dry_run {
            tracing::info!(?signal, "dry run: skipping execution");
            return Ok(());
        }
        match signal {
            Signal::Arbitrage(_) => {
                tracing::info!("would execute arbitrage");
            }
            Signal::Quote(q) => {
                tracing::info!(?q, "would quote");
            }
            Signal::Cancel(id) => {
                tracing::info!(%id, "would cancel");
            }
        }
        Ok(())
    }

    pub async fn route_order(&mut self, order: &NewOrder) -> Result<(), PmError> {
        if self.risk.is_killed() {
            return Err(PmError::kill_switch("trading halted"));
        }
        self.risk.check_order(order).await?;
        if self.dry_run {
            tracing::info!(?order, "dry run: skipping order");
            return Ok(());
        }
        tracing::info!(?order, "would place order");
        Ok(())
    }

    pub fn record_fill(&mut self, fill: Fill) {
        self.fills.push(fill);
    }
}

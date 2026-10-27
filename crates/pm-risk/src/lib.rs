use async_trait::async_trait;
use dashmap::DashMap;
use pm_core::errors::PmError;
use pm_core::traits::RiskCheck;
use pm_core::types::{NewOrder, Signal};
use pm_core::RiskConfig;
use rust_decimal::Decimal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct RiskEngine {
    pub config: RiskConfig,
    pub kill_switch: Arc<AtomicBool>,
    pub open_orders: DashMap<String, ()>,
    pub positions: DashMap<String, Decimal>, // market id -> quantity
    pub total_exposure: Arc<std::sync::Mutex<Decimal>>,
    pub realized_pnl: Arc<std::sync::Mutex<Decimal>>,
    pub peak_pnl: Arc<std::sync::Mutex<Decimal>>,
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            kill_switch: Arc::new(AtomicBool::new(false)),
            open_orders: DashMap::new(),
            positions: DashMap::new(),
            total_exposure: Arc::new(std::sync::Mutex::new(Decimal::ZERO)),
            realized_pnl: Arc::new(std::sync::Mutex::new(Decimal::ZERO)),
            peak_pnl: Arc::new(std::sync::Mutex::new(Decimal::ZERO)),
        }
    }

    pub fn is_killed(&self) -> bool {
        self.kill_switch.load(Ordering::SeqCst)
    }

    pub fn trigger_kill_switch(&self, reason: &str) {
        self.kill_switch.store(true, Ordering::SeqCst);
        tracing::error!("kill switch triggered: {}", reason);
    }

    pub fn check_drawdown(&self) -> Result<(), PmError> {
        if self.is_killed() {
            return Err(PmError::kill_switch("trading halted"));
        }
        let pnl = *self.realized_pnl.lock().unwrap();
        let mut peak = self.peak_pnl.lock().unwrap();
        if pnl > *peak {
            *peak = pnl;
        }
        let drawdown = *peak - pnl;
        let threshold = Decimal::from(self.config.max_drawdown_cents) / Decimal::from(100);
        if drawdown >= threshold {
            self.trigger_kill_switch("max drawdown exceeded");
            return Err(PmError::kill_switch("max drawdown exceeded"));
        }
        Ok(())
    }
}

#[async_trait]
impl RiskCheck for RiskEngine {
    async fn check_order(&self, order: &NewOrder) -> Result<(), PmError> {
        if self.is_killed() {
            return Err(PmError::kill_switch("trading halted"));
        }
        self.check_drawdown()?;
        // open orders limit
        if self.open_orders.len() as u64 >= self.config.max_open_orders {
            return Err(PmError::risk_limit_exceeded("max open orders"));
        }
        // position per market limit
        let key = order.market_id.id.clone();
        let current = self
            .positions
            .get(&key)
            .map(|v| *v)
            .unwrap_or(Decimal::ZERO);
        let new_qty = current + order.quantity;
        let limit = Decimal::from(self.config.max_position_per_market);
        if new_qty > limit {
            return Err(PmError::risk_limit_exceeded("max position per market"));
        }
        // total exposure
        let mut exp = self.total_exposure.lock().unwrap();
        let add = order.quantity * order.price.unwrap_or(Decimal::ONE);
        if *exp + add > Decimal::from(self.config.max_total_exposure) {
            return Err(PmError::risk_limit_exceeded("max total exposure"));
        }
        *exp += add;
        Ok(())
    }

    async fn check_signal(&self, signal: &Signal) -> Result<(), PmError> {
        if self.is_killed() {
            return Err(PmError::kill_switch("trading halted"));
        }
        // Basic: allow all unless killed
        let _ = signal;
        Ok(())
    }
}

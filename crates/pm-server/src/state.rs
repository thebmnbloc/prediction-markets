use axum::extract::FromRef;
use pm_core::AppConfig;
use pm_execution::ExecutionEngine;
use pm_persistence::TradeStore;
use pm_polymarket::PolymarketAdapter;
use pm_risk::RiskEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SignalRecord {
    pub id: String,
    pub market_id: String,
    pub title: String,
    pub side: String,
    pub executable_price: String,
    pub model_probability: String,
    pub net_ev: String,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CrossMarketLink {
    pub id: String,
    pub market_a_id: String,
    pub market_b_id: String,
    pub relation_type: String,
    pub settlement_compatible: bool,
    pub review_status: String,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaperOrder {
    pub id: String,
    pub market_id: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub status: String,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub polymarket: Arc<PolymarketAdapter>,
    pub risk: Arc<RiskEngine>,
    pub execution: Arc<tokio::sync::Mutex<ExecutionEngine>>,
    pub signals: Arc<tokio::sync::Mutex<Vec<SignalRecord>>>,
    pub paper_orders: Arc<tokio::sync::Mutex<Vec<PaperOrder>>>,
    pub cross_links: Arc<tokio::sync::Mutex<Vec<CrossMarketLink>>>,
    pub store: Option<Arc<TradeStore>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let polymarket = Arc::new(PolymarketAdapter::from_app_config(&config));
        let risk = Arc::new(RiskEngine::new(config.risk.clone()));
        let execution = Arc::new(tokio::sync::Mutex::new(ExecutionEngine::new(
            config.general.dry_run,
            config.risk.clone(),
        )));
        let store = std::env::var("DATABASE_URL")
            .ok()
            .and_then(|url| {
                // best-effort; block on connection
                tokio::runtime::Handle::current()
                    .block_on(TradeStore::new(&url))
                    .ok()
            })
            .map(Arc::new);
        Self {
            config: Arc::new(config),
            polymarket,
            risk,
            execution,
            signals: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            paper_orders: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            cross_links: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            store,
        }
    }
}

impl FromRef<AppState> for Arc<AppConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

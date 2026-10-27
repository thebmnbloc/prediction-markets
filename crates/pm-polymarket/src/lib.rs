//! Polymarket adapter for the prediction markets quant system.

use async_trait::async_trait;
use pm_core::errors::PmError;
use pm_core::traits::ExchangeAdapter;
use pm_core::types::{
    Balance, Exchange, Market, MarketFilter, MarketId, NewOrder, OrderBook, OrderConfirmation,
    Position,
};
use pm_core::{AppConfig, PolymarketConfig};
use reqwest::Client;
use std::str::FromStr;

pub struct PolymarketAdapter {
    config: PolymarketConfig,
    client: Client,
}

impl PolymarketAdapter {
    pub fn new(config: PolymarketConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    pub fn from_app_config(cfg: &AppConfig) -> Self {
        Self::new(cfg.polymarket.clone())
    }
    pub fn base_url(&self) -> &str {
        self.config.clob_url()
    }
    pub fn gamma_url(&self) -> &str {
        self.config.gamma_url()
    }
}

#[async_trait]
impl ExchangeAdapter for PolymarketAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::Polymarket
    }

    async fn get_markets(&self, _filter: &MarketFilter) -> Result<Vec<Market>, PmError> {
        let url = format!("{}/markets", self.gamma_url());
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PmError::network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PmError::exchange_api_status(
                Exchange::Polymarket,
                "get_markets failed".to_string(),
                resp.status().as_u16(),
            ));
        }
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PmError::serialization(e.to_string()))?;
        let mut markets = Vec::new();
        let arr = value
            .as_array()
            .cloned()
            .or_else(|| value.get("data").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();
        for m in arr {
            let id = m
                .get("conditionId")
                .or_else(|| m.get("condition_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = m
                .get("question")
                .or_else(|| m.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let volume = m.get("volume").and_then(|v| v.as_str()).unwrap_or("0");
            let volume =
                rust_decimal::Decimal::from_str(volume).unwrap_or(rust_decimal::Decimal::ZERO);
            markets.push(Market {
                id: MarketId::new(Exchange::Polymarket, id),
                title,
                category: pm_core::types::Category::Other("unknown".to_string()),
                outcomes: vec![],
                status: pm_core::types::MarketStatus::Open,
                expiration: None,
                volume,
                last_updated: chrono::Utc::now(),
            });
        }
        Ok(markets)
    }

    async fn get_market(&self, market_id: &str) -> Result<Market, PmError> {
        let url = format!(
            "{}/markets?condition_ids={}&limit=1",
            self.gamma_url(),
            market_id
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PmError::network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PmError::exchange_api_status(
                Exchange::Polymarket,
                "get_market failed".to_string(),
                resp.status().as_u16(),
            ));
        }
        let arr: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PmError::serialization(e.to_string()))?;
        let m = arr.get(0).cloned().unwrap_or(serde_json::Value::Null);
        let title = m
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or(market_id)
            .to_string();
        Ok(Market {
            id: MarketId::new(Exchange::Polymarket, market_id),
            title,
            category: pm_core::types::Category::Other("unknown".to_string()),
            outcomes: vec![],
            status: pm_core::types::MarketStatus::Open,
            expiration: None,
            volume: rust_decimal::Decimal::ZERO,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn get_orderbook(&self, market_id: &str) -> Result<OrderBook, PmError> {
        let url = format!(
            "{}/markets?condition_ids={}&limit=1",
            self.gamma_url(),
            market_id
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PmError::network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PmError::exchange_api_status(
                Exchange::Polymarket,
                "get_orderbook market lookup failed".to_string(),
                resp.status().as_u16(),
            ));
        }
        let arr: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PmError::serialization(e.to_string()))?;
        let token_id = arr
            .get(0)
            .and_then(|m| m.get("clobTokenIds"))
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .and_then(|a| a.into_iter().next())
            .unwrap_or_else(|| market_id.to_string());
        let book_url = format!("{}/book?token_id={}", self.base_url(), token_id);
        let resp = self
            .client
            .get(&book_url)
            .send()
            .await
            .map_err(|e| PmError::network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PmError::exchange_api_status(
                Exchange::Polymarket,
                format!(
                    "get_orderbook failed token={} status={}",
                    token_id,
                    resp.status()
                ),
                resp.status().as_u16(),
            ));
        }
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PmError::serialization(e.to_string()))?;
        use std::str::FromStr;
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        if let Some(arr) = v.get("bids").and_then(|x| x.as_array()) {
            for level in arr {
                let price = level.get("price").and_then(|x| x.as_str()).unwrap_or("0");
                let size = level.get("size").and_then(|x| x.as_str()).unwrap_or("0");
                bids.push(pm_core::types::OrderBookLevel {
                    price: rust_decimal::Decimal::from_str(price)
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                    quantity: rust_decimal::Decimal::from_str(size)
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                });
            }
        }
        if let Some(arr) = v.get("asks").and_then(|x| x.as_array()) {
            for level in arr {
                let price = level.get("price").and_then(|x| x.as_str()).unwrap_or("0");
                let size = level.get("size").and_then(|x| x.as_str()).unwrap_or("0");
                asks.push(pm_core::types::OrderBookLevel {
                    price: rust_decimal::Decimal::from_str(price)
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                    quantity: rust_decimal::Decimal::from_str(size)
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                });
            }
        }
        Ok(OrderBook {
            market_id: MarketId::new(Exchange::Polymarket, market_id),
            bids,
            asks,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn place_order(&self, order: &NewOrder) -> Result<OrderConfirmation, PmError> {
        if self.config.private_key.is_none() {
            return Err(PmError::auth(Exchange::Polymarket, "missing private key"));
        }
        Ok(OrderConfirmation {
            order_id: "pm-order".to_string(),
            exchange: Exchange::Polymarket,
            market_id: order.market_id.clone(),
            status: pm_core::types::OrderStatus::Open,
            filled_quantity: rust_decimal::Decimal::ZERO,
            avg_fill_price: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn cancel_order(&self, _order_id: &str) -> Result<(), PmError> {
        Ok(())
    }

    async fn get_positions(&self) -> Result<Vec<Position>, PmError> {
        Ok(vec![])
    }

    async fn get_balance(&self) -> Result<Balance, PmError> {
        Ok(Balance {
            exchange: Exchange::Polymarket,
            available: rust_decimal::Decimal::ZERO,
            reserved: rust_decimal::Decimal::ZERO,
            total: rust_decimal::Decimal::ZERO,
        })
    }
}

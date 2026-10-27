//! Application configuration types and loading logic.
//!
//! Loads settings from `config/default.toml` and overlays them with environment variables.

use config::{Config, Environment, File};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::errors::PmError;

/// Root configuration for the prediction markets trading system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppConfig {
    /// General execution and logging settings.
    pub general: GeneralConfig,
    /// Kalshi exchange connection configuration.
    pub kalshi: KalshiConfig,
    /// Polymarket exchange connection configuration.
    pub polymarket: PolymarketConfig,
    /// Strategy configurations.
    pub strategies: StrategiesConfig,
    /// Risk limits and kill switch triggers.
    pub risk: RiskConfig,
    /// REST and WebSocket API server settings.
    pub server: ServerConfig,
}

/// General system options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralConfig {
    /// If true, orders are simulated or skipped rather than sent to real markets.
    pub dry_run: bool,
    /// Logging filter level (e.g. "debug", "info", "warn", "error").
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            dry_run: true,
            log_level: "info".to_string(),
        }
    }
}

/// Kalshi API connection settings.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KalshiConfig {
    /// Target environment: "demo" or "production".
    pub env: String,
    /// Production REST API base URL.
    pub rest_base_url_prod: String,
    /// Demo / sandbox REST API base URL.
    pub rest_base_url_demo: String,
    /// Production WebSocket URL.
    pub ws_url_prod: String,
    /// Demo / sandbox WebSocket URL.
    pub ws_url_demo: String,
    /// Optional API key UUID loaded from environment (`KALSHI_API_KEY_ID`).
    #[serde(default)]
    pub api_key_id: Option<String>,
    /// Optional path to RSA private key PEM file (`KALSHI_PRIVATE_KEY_PATH`).
    #[serde(default)]
    pub private_key_path: Option<String>,
}

impl Default for KalshiConfig {
    fn default() -> Self {
        Self {
            env: "demo".to_string(),
            rest_base_url_prod: "https://external-api.kalshi.com/trade-api/v2".to_string(),
            rest_base_url_demo: "https://external-api.demo.kalshi.co/trade-api/v2".to_string(),
            ws_url_prod: "wss://external-api-ws.kalshi.com/trade-api/ws/v2".to_string(),
            ws_url_demo: "wss://external-api-ws.demo.kalshi.co/trade-api/ws/v2".to_string(),
            api_key_id: None,
            private_key_path: None,
        }
    }
}

impl KalshiConfig {
    /// Returns true if targeting the production Kalshi environment.
    pub fn is_production(&self) -> bool {
        self.env.eq_ignore_ascii_case("production") || self.env.eq_ignore_ascii_case("prod")
    }

    /// Select the appropriate REST base URL based on `env`.
    pub fn rest_base_url(&self) -> &str {
        if self.is_production() {
            &self.rest_base_url_prod
        } else {
            &self.rest_base_url_demo
        }
    }

    /// Select the appropriate WebSocket URL based on `env`.
    pub fn ws_url(&self) -> &str {
        if self.is_production() {
            &self.ws_url_prod
        } else {
            &self.ws_url_demo
        }
    }
}

/// Polymarket API connection settings.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolymarketConfig {
    /// Target environment: "staging" or "production".
    pub env: String,
    /// Production CLOB REST API base URL.
    pub clob_base_url: String,
    /// Staging CLOB REST API base URL.
    pub clob_staging_url: String,
    /// Gamma Markets API base URL.
    pub gamma_base_url: String,
    /// CLOB WebSocket URL.
    pub ws_url: String,
    /// Optional Ethereum/Polygon wallet private key hex (`POLYMARKET_PRIVATE_KEY`).
    #[serde(default, skip_serializing)]
    pub private_key: Option<String>,
}

impl Default for PolymarketConfig {
    fn default() -> Self {
        Self {
            env: "production".to_string(),
            clob_base_url: "https://clob.polymarket.com".to_string(),
            clob_staging_url: "https://clob-staging.polymarket.com".to_string(),
            gamma_base_url: "https://gamma-api.polymarket.com".to_string(),
            ws_url: "wss://ws-subscriptions-clob.polymarket.com/ws/market".to_string(),
            private_key: None,
        }
    }
}

impl PolymarketConfig {
    /// Returns true if targeting the production Polymarket environment.
    pub fn is_production(&self) -> bool {
        self.env.eq_ignore_ascii_case("production") || self.env.eq_ignore_ascii_case("prod")
    }

    /// Select the CLOB REST URL based on `env`.
    pub fn clob_url(&self) -> &str {
        if self.is_production() {
            &self.clob_base_url
        } else {
            &self.clob_staging_url
        }
    }

    /// Return the Gamma API base URL.
    pub fn gamma_url(&self) -> &str {
        &self.gamma_base_url
    }

    /// Return the WebSocket URL.
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }
}

/// Configuration container for all trading strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StrategiesConfig {
    /// Cross-exchange arbitrage strategy settings.
    pub cross_arb: CrossArbConfig,
    /// Bundle / multi-outcome arbitrage strategy settings.
    pub bundle_arb: BundleArbConfig,
    /// Market maker strategy settings.
    pub market_maker: MarketMakerConfig,
}

/// Cross-exchange arbitrage parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossArbConfig {
    /// Whether cross-market arbitrage detection is active.
    pub enabled: bool,
    /// Minimum edge in cents (or basis points) required to trigger execution.
    pub min_edge_cents: u64,
    /// Maximum contract position per individual market.
    pub max_position_per_market: u64,
    /// Interval in milliseconds between re-evaluations.
    pub refresh_interval_ms: u64,
}

impl Default for CrossArbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_edge_cents: 50,
            max_position_per_market: 100,
            refresh_interval_ms: 500,
        }
    }
}

/// Bundle arbitrage parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleArbConfig {
    /// Whether bundle arbitrage is enabled.
    pub enabled: bool,
    /// Minimum edge in cents required to trigger execution.
    pub min_edge_cents: u64,
}

impl Default for BundleArbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_edge_cents: 30,
        }
    }
}

/// Two-sided market making parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketMakerConfig {
    /// Whether the market maker strategy is enabled.
    pub enabled: bool,
    /// Target bid-ask spread width in cents.
    pub spread_width_cents: u64,
    /// Maximum inventory position per contract.
    pub max_position: u64,
    /// Inventory skew coefficient (Decimal).
    pub skew_factor: Decimal,
    /// Interval in milliseconds between quote refreshes.
    pub refresh_interval_ms: u64,
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            spread_width_cents: 300,
            max_position: 50,
            skew_factor: dec!(0.5),
            refresh_interval_ms: 1000,
        }
    }
}

/// Risk management limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskConfig {
    /// Maximum total portfolio exposure in cents.
    pub max_total_exposure: u64,
    /// Maximum position allowed in a single market in cents.
    pub max_position_per_market: u64,
    /// Drawdown threshold in cents that triggers an automated kill switch.
    pub max_drawdown_cents: u64,
    /// Maximum number of concurrent open limit orders.
    pub max_open_orders: u64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_total_exposure: 10000,
            max_position_per_market: 500,
            max_drawdown_cents: 2000,
            max_open_orders: 50,
        }
    }
}

/// API server configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    /// Host interface to bind server.
    pub host: String,
    /// Port number to listen on.
    pub port: u16,
    /// Allowed CORS origin header.
    pub cors_origin: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3001,
            cors_origin: "http://localhost:3000".to_string(),
        }
    }
}

impl AppConfig {
    /// Locate candidate default configuration path.
    pub fn find_default_config_path() -> PathBuf {
        if let Ok(path_str) = std::env::var("PM_CONFIG_PATH") {
            let p = PathBuf::from(path_str);
            if p.exists() {
                return p;
            }
        }
        let candidates = [
            "config/default.toml",
            "../config/default.toml",
            "../../config/default.toml",
        ];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return p;
            }
        }
        PathBuf::from("config/default.toml")
    }

    /// Load application configuration from default paths overlaid with environment variables.
    pub fn load() -> Result<Self, PmError> {
        let path = Self::find_default_config_path();
        Self::load_from_path(path)
    }

    /// Load configuration from a specific file path, with environment variable overrides.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, PmError> {
        // Load .env if present
        dotenvy::dotenv().ok();

        let mut builder = Config::builder();

        if path.as_ref().exists() {
            builder = builder.add_source(File::from(path.as_ref()));
        } else {
            // If file doesn't exist, serialize default into source so defaults are present
            let default_cfg = AppConfig::default();
            let toml_str = config_as_json_str(&default_cfg)?;
            builder =
                builder.add_source(config::File::from_str(&toml_str, config::FileFormat::Json));
        }

        // Overlay with PM__ prefixed environment variables
        builder = builder.add_source(
            Environment::with_prefix("PM")
                .separator("__")
                .try_parsing(true),
        );

        let config_obj = builder.build()?;
        let mut cfg: AppConfig = config_obj.try_deserialize()?;

        // Apply standard environment variables (e.g. from .env.example)
        Self::apply_env_overrides(&mut cfg);

        info!(
            dry_run = cfg.general.dry_run,
            kalshi_env = %cfg.kalshi.env,
            polymarket_env = %cfg.polymarket.env,
            "Loaded application configuration"
        );

        Ok(cfg)
    }

    /// Apply explicit environment variable overrides matching `.env.example`.
    fn apply_env_overrides(cfg: &mut AppConfig) {
        if let Ok(v) = std::env::var("DRY_RUN") {
            if let Ok(b) = v.parse::<bool>() {
                cfg.general.dry_run = b;
            }
        }
        if let Ok(v) = std::env::var("LOG_LEVEL") {
            cfg.general.log_level = v;
        }
        if let Ok(v) = std::env::var("KALSHI_ENV") {
            cfg.kalshi.env = v;
        }
        if let Ok(v) = std::env::var("KALSHI_API_KEY_ID") {
            cfg.kalshi.api_key_id = Some(v);
        }
        if let Ok(v) = std::env::var("KALSHI_PRIVATE_KEY_PATH") {
            cfg.kalshi.private_key_path = Some(v);
        }
        if let Ok(v) = std::env::var("POLYMARKET_ENV") {
            cfg.polymarket.env = v;
        }
        if let Ok(v) = std::env::var("POLYMARKET_PRIVATE_KEY") {
            cfg.polymarket.private_key = Some(v);
        }
        if let Ok(v) = std::env::var("SERVER_HOST") {
            cfg.server.host = v;
        }
        if let Ok(v) = std::env::var("SERVER_PORT") {
            if let Ok(p) = v.parse::<u16>() {
                cfg.server.port = p;
            }
        }
    }
}

fn config_as_json_str(cfg: &AppConfig) -> Result<String, PmError> {
    serde_json::to_string(cfg).map_err(PmError::from)
}

impl std::fmt::Debug for KalshiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KalshiConfig")
            .field("env", &self.env)
            .field("rest_base_url_prod", &self.rest_base_url_prod)
            .field("rest_base_url_demo", &self.rest_base_url_demo)
            .field("ws_url_prod", &self.ws_url_prod)
            .field("ws_url_demo", &self.ws_url_demo)
            .field("api_key_id", &self.api_key_id)
            .field("private_key_path", &self.private_key_path)
            .finish()
    }
}

impl std::fmt::Debug for PolymarketConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolymarketConfig")
            .field("env", &self.env)
            .field("clob_base_url", &self.clob_base_url)
            .field("clob_staging_url", &self.clob_staging_url)
            .field("gamma_base_url", &self.gamma_base_url)
            .field("ws_url", &self.ws_url)
            .field("private_key", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalshi_url_selection() {
        let k = KalshiConfig {
            env: "demo".to_string(),
            ..KalshiConfig::default()
        };
        assert!(!k.is_production());
        assert_eq!(k.rest_base_url(), &k.rest_base_url_demo);
        assert_eq!(k.ws_url(), &k.ws_url_demo);

        let k = KalshiConfig {
            env: "production".to_string(),
            ..KalshiConfig::default()
        };
        assert!(k.is_production());
        assert_eq!(k.rest_base_url(), &k.rest_base_url_prod);
        assert_eq!(k.ws_url(), &k.ws_url_prod);
    }

    #[test]
    fn test_polymarket_url_selection() {
        let p = PolymarketConfig {
            env: "staging".to_string(),
            ..PolymarketConfig::default()
        };
        assert!(!p.is_production());
        assert_eq!(p.clob_url(), &p.clob_staging_url);

        let p = PolymarketConfig {
            env: "production".to_string(),
            ..PolymarketConfig::default()
        };
        assert!(p.is_production());
        assert_eq!(p.clob_url(), &p.clob_base_url);
    }

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert!(cfg.general.dry_run);
        assert_eq!(cfg.server.port, 3001);
        assert_eq!(cfg.strategies.cross_arb.min_edge_cents, 50);
        assert_eq!(cfg.risk.max_drawdown_cents, 2000);
    }

    #[test]
    fn test_env_override() {
        let mut cfg = AppConfig::default();
        std::env::set_var("KALSHI_API_KEY_ID", "test-uuid-1234");
        std::env::set_var("DRY_RUN", "false");

        AppConfig::apply_env_overrides(&mut cfg);

        assert_eq!(cfg.kalshi.api_key_id.as_deref(), Some("test-uuid-1234"));
        assert!(!cfg.general.dry_run);

        std::env::remove_var("KALSHI_API_KEY_ID");
        std::env::remove_var("DRY_RUN");
    }
}

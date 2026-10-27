//! Error types for the prediction markets quant trading system.

use crate::types::Exchange;

/// Unified error enum for all prediction market operations.
#[derive(Debug, thiserror::Error)]
pub enum PmError {
    /// Failure response from an exchange REST or GraphQL API.
    #[error("Exchange API error ({exchange}): {message}")]
    ExchangeApi {
        /// Exchange returning the error.
        exchange: Exchange,
        /// Description of the error.
        message: String,
        /// HTTP status code if returned by REST API.
        status_code: Option<u16>,
    },

    /// Authentication or credential signing failure.
    #[error("Authentication error ({exchange}): {message}")]
    Auth {
        /// Target exchange.
        exchange: Exchange,
        /// Error description.
        message: String,
    },

    /// WebSocket connectivity or frame handling error.
    #[error("WebSocket error ({exchange}): {message}")]
    WebSocket {
        /// Target exchange.
        exchange: Exchange,
        /// Error description.
        message: String,
    },

    /// Order submission rejected by exchange or validation layer.
    #[error("Order rejected: {reason}")]
    OrderRejected {
        /// Reason for rejection.
        reason: String,
    },

    /// Pre-trade or post-trade risk check violation.
    #[error("Risk limit exceeded: {limit}")]
    RiskLimitExceeded {
        /// Description of violated risk limit.
        limit: String,
    },

    /// Specified market ticker or condition ID was not found.
    #[error("Market not found: {market_id}")]
    MarketNotFound {
        /// Market ticker or condition ID.
        market_id: String,
    },

    /// Configuration file parsing or missing setting error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// SQLite / persistence storage failure.
    #[error("Database error: {0}")]
    Database(String),

    /// JSON / format serialization or deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// HTTP client, connection, or DNS resolution failure.
    #[error("Network error: {0}")]
    Network(String),

    /// System kill switch triggered, halting all trading actions.
    #[error("Kill switch activated: {reason}")]
    KillSwitch {
        /// Trigger cause.
        reason: String,
    },

    /// Unspecified internal or runtime error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PmError {
    /// Construct an `ExchangeApi` error with an optional HTTP status code.
    pub fn exchange_api(
        exchange: Exchange,
        message: impl Into<String>,
        status_code: Option<u16>,
    ) -> Self {
        PmError::ExchangeApi {
            exchange,
            message: message.into(),
            status_code,
        }
    }

    /// Construct an `ExchangeApi` error without a status code.
    pub fn exchange_api_msg(exchange: Exchange, message: impl Into<String>) -> Self {
        Self::exchange_api(exchange, message, None)
    }

    /// Construct an `ExchangeApi` error with a specific HTTP status code.
    pub fn exchange_api_status(
        exchange: Exchange,
        message: impl Into<String>,
        status_code: u16,
    ) -> Self {
        Self::exchange_api(exchange, message, Some(status_code))
    }

    /// Construct an `Auth` error for the specified exchange.
    pub fn auth(exchange: Exchange, message: impl Into<String>) -> Self {
        PmError::Auth {
            exchange,
            message: message.into(),
        }
    }

    /// Construct a `WebSocket` error for the specified exchange.
    pub fn websocket(exchange: Exchange, message: impl Into<String>) -> Self {
        PmError::WebSocket {
            exchange,
            message: message.into(),
        }
    }

    /// Construct an `OrderRejected` error.
    pub fn order_rejected(reason: impl Into<String>) -> Self {
        PmError::OrderRejected {
            reason: reason.into(),
        }
    }

    /// Construct a `RiskLimitExceeded` error.
    pub fn risk_limit_exceeded(limit: impl Into<String>) -> Self {
        PmError::RiskLimitExceeded {
            limit: limit.into(),
        }
    }

    /// Construct a `MarketNotFound` error.
    pub fn market_not_found(market_id: impl Into<String>) -> Self {
        PmError::MarketNotFound {
            market_id: market_id.into(),
        }
    }

    /// Construct a `KillSwitch` error.
    pub fn kill_switch(reason: impl Into<String>) -> Self {
        PmError::KillSwitch {
            reason: reason.into(),
        }
    }

    /// Construct a `Config` error.
    pub fn config(msg: impl Into<String>) -> Self {
        PmError::Config(msg.into())
    }

    /// Construct a `Database` error.
    pub fn database(msg: impl Into<String>) -> Self {
        PmError::Database(msg.into())
    }

    /// Construct a `Serialization` error.
    pub fn serialization(msg: impl Into<String>) -> Self {
        PmError::Serialization(msg.into())
    }

    /// Construct a `Network` error.
    pub fn network(msg: impl Into<String>) -> Self {
        PmError::Network(msg.into())
    }

    /// Construct an `Internal` error.
    pub fn internal(msg: impl Into<String>) -> Self {
        PmError::Internal(msg.into())
    }
}

impl From<reqwest::Error> for PmError {
    fn from(err: reqwest::Error) -> Self {
        PmError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for PmError {
    fn from(err: serde_json::Error) -> Self {
        PmError::Serialization(err.to_string())
    }
}

impl From<config::ConfigError> for PmError {
    fn from(err: config::ConfigError) -> Self {
        PmError::Config(err.to_string())
    }
}

impl From<std::io::Error> for PmError {
    fn from(err: std::io::Error) -> Self {
        PmError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PmError::exchange_api(Exchange::Kalshi, "Rate limit exceeded", Some(429));
        assert_eq!(
            err.to_string(),
            "Exchange API error (kalshi): Rate limit exceeded"
        );

        let auth_err = PmError::auth(Exchange::Polymarket, "Invalid signature");
        assert_eq!(
            auth_err.to_string(),
            "Authentication error (polymarket): Invalid signature"
        );

        let risk_err = PmError::risk_limit_exceeded("Max drawdown $20 breached");
        assert_eq!(
            risk_err.to_string(),
            "Risk limit exceeded: Max drawdown $20 breached"
        );

        let kill_err = PmError::kill_switch("Manual emergency stop");
        assert_eq!(
            kill_err.to_string(),
            "Kill switch activated: Manual emergency stop"
        );
    }

    #[test]
    fn test_from_serde_json_error() {
        let bad_json = "{ invalid_json ";
        let json_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
        let pm_err: PmError = json_err.into();

        match pm_err {
            PmError::Serialization(msg) => assert!(!msg.is_empty()),
            other => panic!("Expected Serialization error, got: {other:?}"),
        }
    }

    #[test]
    fn test_from_config_error() {
        let cfg_err = config::ConfigError::NotFound("some_key".to_string());
        let pm_err: PmError = cfg_err.into();

        match pm_err {
            PmError::Config(msg) => assert!(msg.contains("some_key")),
            other => panic!("Expected Config error, got: {other:?}"),
        }
    }
}

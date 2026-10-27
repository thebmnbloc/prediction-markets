use thiserror::Error;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("inconsistent book for {symbol}: {reason}")]
    Inconsistent { symbol: String, reason: String },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("output error: {0}")]
    Output(String),
}

pub type Result<T> = std::result::Result<T, CollectorError>;

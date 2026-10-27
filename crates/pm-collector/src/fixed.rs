//! Exact fixed-point conversion for the challenge's `int64` fields.
//!
//! The challenge schema stores prices and quantities with 6 implied decimal
//! places (`1_000_000` = 1.00 USDC or 1 contract). Kalshi sends decimal
//! strings, so values are parsed as `Decimal` and never pass through `f64`.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::error::{CollectorError, Result};

pub const SCALE: i64 = 1_000_000;

/// Parse a decimal string such as `"0.0800"` or `"-54.00"` into fixed point.
/// Rejects values with more than six decimal places instead of rounding.
pub fn parse_fixed(text: &str) -> Result<i64> {
    let value = Decimal::from_str(text.trim())
        .map_err(|e| CollectorError::Parse(format!("invalid decimal {text:?}: {e}")))?;
    let scaled = value * Decimal::from(SCALE);
    if scaled.fract() != Decimal::ZERO {
        return Err(CollectorError::Parse(format!(
            "{text:?} has more than 6 decimal places"
        )));
    }
    scaled
        .trunc()
        .to_i64()
        .ok_or_else(|| CollectorError::Parse(format!("{text:?} is out of range")))
}

/// Parse a YES-side price in dollars. Must be strictly between 0 and 1.
pub fn parse_price(text: &str) -> Result<i64> {
    let price = parse_fixed(text)?;
    if price <= 0 || price >= SCALE {
        return Err(CollectorError::Parse(format!(
            "price {text:?} must be strictly between 0 and 1"
        )));
    }
    Ok(price)
}

/// Parse a contract count. Counts are never negative.
pub fn parse_count(text: &str) -> Result<i64> {
    let count = parse_fixed(text)?;
    if count < 0 {
        return Err(CollectorError::Parse(format!("count {text:?} is negative")));
    }
    Ok(count)
}

/// The NO price that corresponds to a YES price on the other side of the book.
pub fn complement(price: i64) -> i64 {
    SCALE - price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kalshi_price_and_count_formats() {
        assert_eq!(parse_fixed("0.0800").unwrap(), 80_000);
        assert_eq!(parse_fixed("300.00").unwrap(), 300_000_000);
        assert_eq!(parse_fixed("-54.00").unwrap(), -54_000_000);
    }

    #[test]
    fn rejects_excess_precision_instead_of_rounding() {
        assert!(parse_fixed("0.0000001").is_err());
    }

    #[test]
    fn rejects_prices_outside_open_interval() {
        assert!(parse_price("0").is_err());
        assert!(parse_price("1.0000").is_err());
        assert_eq!(parse_price("0.5400").unwrap(), 540_000);
    }

    #[test]
    fn rejects_negative_counts() {
        assert!(parse_count("-1.00").is_err());
    }

    #[test]
    fn complement_is_exact() {
        assert_eq!(complement(540_000), 460_000);
    }
}

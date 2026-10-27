//! One market's order book in fixed point.
//!
//! Kalshi publishes a YES book and a NO book. Both are bids. A NO bid at
//! price `p` is the same position as a YES offer at `1 - p`, so the collector
//! stores YES bids as `bids` and NO bids as `asks` at `1 - p`. This gives a
//! single YES-denominated book with bids and asks, which is what the challenge
//! schema expects.

use std::collections::BTreeMap;

use crate::error::{CollectorError, Result};
use crate::fixed::{complement, parse_count, parse_price};
use crate::kalshi::{DeltaBody, SnapshotBody, WireSide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookSide {
    Bid,
    Ask,
}

/// One price level after a delta, with its absolute quantity.
/// `quantity == 0` means the level was removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelUpdate {
    pub symbol: String,
    pub side: BookSide,
    pub price: i64,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub symbol: String,
    /// price -> absolute quantity (YES bids)
    pub bids: BTreeMap<i64, i64>,
    /// price -> absolute quantity (NO bids, expressed as YES asks)
    pub asks: BTreeMap<i64, i64>,
    /// Exchange clock of the last change applied to this book.
    pub clock: u64,
}

fn inconsistent(symbol: &str, reason: impl Into<String>) -> CollectorError {
    CollectorError::Inconsistent {
        symbol: symbol.to_owned(),
        reason: reason.into(),
    }
}

fn parse_levels(symbol: &str, levels: &[[String; 2]]) -> Result<BTreeMap<i64, i64>> {
    let mut out = BTreeMap::new();
    for [price, count] in levels {
        let price = parse_price(price)?;
        let count = parse_count(count)?;
        if count == 0 {
            continue;
        }
        if out.insert(price, count).is_some() {
            return Err(inconsistent(
                symbol,
                format!("duplicate price level {price}"),
            ));
        }
    }
    Ok(out)
}

impl Book {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            clock: 0,
        }
    }

    /// Replace the entire book with a snapshot.
    pub fn from_snapshot(body: &SnapshotBody) -> Result<Self> {
        let bids = parse_levels(&body.market_ticker, &body.yes_dollars_fp)?;
        let no_bids = parse_levels(&body.market_ticker, &body.no_dollars_fp)?;
        let mut asks = BTreeMap::new();
        for (price, count) in no_bids {
            asks.insert(complement(price), count);
        }
        Ok(Self {
            symbol: body.market_ticker.clone(),
            bids,
            asks,
            clock: body.clock,
        })
    }

    /// Apply a signed delta and return the level's new absolute quantity.
    pub fn apply_delta(&mut self, body: &DeltaBody) -> Result<LevelUpdate> {
        let raw_price = parse_price(&body.price_dollars)?;
        let delta = crate::fixed::parse_fixed(&body.delta_fp)?;

        let (side, price, levels) = match body.side {
            WireSide::Yes => (BookSide::Bid, raw_price, &mut self.bids),
            WireSide::No => (BookSide::Ask, complement(raw_price), &mut self.asks),
        };

        let current = levels.get(&price).copied().unwrap_or(0);
        let next = current
            .checked_add(delta)
            .ok_or_else(|| inconsistent(&self.symbol, "quantity overflow"))?;
        if next < 0 {
            return Err(inconsistent(
                &self.symbol,
                format!("level {price} would go negative ({current} + {delta})"),
            ));
        }
        if next == 0 {
            levels.remove(&price);
        } else {
            levels.insert(price, next);
        }
        self.clock = body.clock;

        Ok(LevelUpdate {
            symbol: self.symbol.clone(),
            side,
            price,
            quantity: next,
        })
    }

    /// Best bid price, if any.
    pub fn best_bid(&self) -> Option<i64> {
        self.bids.keys().next_back().copied()
    }

    /// Best ask price, if any.
    pub fn best_ask(&self) -> Option<i64> {
        self.asks.keys().next().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kalshi::WireSide;

    fn snapshot(clock: u64, yes: &[[&str; 2]], no: &[[&str; 2]]) -> SnapshotBody {
        SnapshotBody {
            market_ticker: "KXTEST".into(),
            market_id: None,
            clock,
            yes_dollars_fp: yes
                .iter()
                .map(|[p, c]| [p.to_string(), c.to_string()])
                .collect(),
            no_dollars_fp: no
                .iter()
                .map(|[p, c]| [p.to_string(), c.to_string()])
                .collect(),
        }
    }

    fn delta(clock: u64, price: &str, change: &str, side: WireSide) -> DeltaBody {
        DeltaBody {
            market_ticker: "KXTEST".into(),
            market_id: None,
            clock,
            price_dollars: price.into(),
            delta_fp: change.into(),
            side,
            ts_ms: None,
        }
    }

    #[test]
    fn snapshot_maps_no_bids_to_yes_asks_at_complement() {
        let book = Book::from_snapshot(&snapshot(
            100,
            &[["0.4000", "100.00"]],
            &[["0.5400", "20.00"]],
        ))
        .unwrap();
        assert_eq!(book.bids.get(&400_000), Some(&100_000_000));
        assert_eq!(book.asks.get(&460_000), Some(&20_000_000));
        assert_eq!(book.best_bid(), Some(400_000));
        assert_eq!(book.best_ask(), Some(460_000));
        assert_eq!(book.clock, 100);
    }

    #[test]
    fn delta_writes_absolute_quantity_and_removes_empty_levels() {
        let mut book = Book::from_snapshot(&snapshot(100, &[["0.4000", "100.00"]], &[])).unwrap();

        let up = book
            .apply_delta(&delta(101, "0.4000", "-40.00", WireSide::Yes))
            .unwrap();
        assert_eq!(up.side, BookSide::Bid);
        assert_eq!(up.price, 400_000);
        assert_eq!(up.quantity, 60_000_000);

        let gone = book
            .apply_delta(&delta(102, "0.4000", "-60.00", WireSide::Yes))
            .unwrap();
        assert_eq!(gone.quantity, 0);
        assert!(book.bids.is_empty());
        assert_eq!(book.clock, 102);
    }

    #[test]
    fn no_side_delta_moves_the_yes_ask() {
        let mut book = Book::new("KXTEST");
        let up = book
            .apply_delta(&delta(5, "0.5400", "5.00", WireSide::No))
            .unwrap();
        assert_eq!(up.side, BookSide::Ask);
        assert_eq!(up.price, 460_000);
        assert_eq!(up.quantity, 5_000_000);
    }

    #[test]
    fn negative_result_is_an_inconsistency_not_a_clamp() {
        let mut book = Book::new("KXTEST");
        assert!(book
            .apply_delta(&delta(1, "0.3000", "-1.00", WireSide::Yes))
            .is_err());
    }

    #[test]
    fn duplicate_snapshot_levels_are_rejected() {
        let body = snapshot(1, &[["0.4000", "1.00"], ["0.4000", "2.00"]], &[]);
        assert!(Book::from_snapshot(&body).is_err());
    }
}

//! Kalshi order book collector for the Voluntas engineering challenge.
//!
//! Layers, from pure to effectful:
//! - [`fixed`]: exact decimal → `int64` fixed point.
//! - [`kalshi`]: Kalshi wire messages (parsing only).
//! - [`book`]: one market's book, with absolute levels.
//! - [`engine`]: sequencing, redundant-socket failover, and snapshot cadence.
//! - [`replay`]: deterministic replay of recorded sessions.
//! - [`output`]: writes the challenge's FIX SBE format via the template helpers.

pub mod book;
pub mod engine;
pub mod error;
pub mod fixed;
pub mod kalshi;
pub mod output;
pub mod replay;

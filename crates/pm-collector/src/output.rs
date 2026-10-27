//! Writes engine events in the challenge's FIX SBE format, through the
//! template helpers. Nothing here re-implements the encoding.

use chrono::{DateTime, Utc};
use engineering_challenge::util::dtos::challenge_orderbook::{
    PriceLevelUpdateDomain, SnapshotAsksEntryDomain, SnapshotBidsEntryDomain, SnapshotDomain,
};
use engineering_challenge::util::AlignedSbeWriter;
use engineering_challenge::Side;

use crate::book::BookSide;
use crate::engine::{BookSnapshot, Event};
use crate::error::{CollectorError, Result};

/// Chunk size used by the challenge template's example.
pub const CHUNK_SIZE: usize = 64 << 20;

pub trait EventSink {
    fn write(&mut self, event: &Event) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}

fn output_err(err: anyhow::Error) -> CollectorError {
    CollectorError::Output(format!("{err:#}"))
}

fn timestamp(us: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp_micros(us)
        .ok_or_else(|| CollectorError::Output(format!("timestamp {us} out of range")))
}

fn snapshot_dto(s: &BookSnapshot) -> Result<SnapshotDomain> {
    Ok(SnapshotDomain {
        received_at: timestamp(s.received_at_us)?,
        exchange_timestamp: timestamp(s.exchange_ts_us)?,
        bids: s
            .bids
            .iter()
            .map(|(price_level, quantity)| SnapshotBidsEntryDomain {
                price_level: *price_level,
                quantity: *quantity,
            })
            .collect(),
        asks: s
            .asks
            .iter()
            .map(|(price_level, quantity)| SnapshotAsksEntryDomain {
                price_level: *price_level,
                quantity: *quantity,
            })
            .collect(),
        symbol: s.symbol.clone(),
    })
}

/// Writes events to a challenge-format file via `AlignedSbeWriter`.
pub struct SbeSink {
    writer: AlignedSbeWriter,
}

impl SbeSink {
    pub fn create(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let writer = AlignedSbeWriter::create(path, CHUNK_SIZE).map_err(output_err)?;
        Ok(Self { writer })
    }
}

impl EventSink for SbeSink {
    fn write(&mut self, event: &Event) -> Result<()> {
        match event {
            Event::Snapshot(snapshot) => {
                let dto = snapshot_dto(snapshot)?;
                self.writer.write_msg(&dto).map_err(output_err)
            }
            Event::Level {
                received_at_us,
                exchange_ts_us,
                update,
            } => {
                let dto = PriceLevelUpdateDomain {
                    backtest_timestamp: timestamp(*received_at_us)?,
                    exchange_timestamp: timestamp(*exchange_ts_us)?,
                    price_level: update.price,
                    quantity: update.quantity,
                    side: match update.side {
                        BookSide::Bid => Side::Buy,
                        BookSide::Ask => Side::Sell,
                    },
                    symbol: update.symbol.clone(),
                };
                self.writer.write_msg(&dto).map_err(output_err)
            }
        }
    }

    fn finish(&mut self) -> Result<()> {
        self.writer.flush().map_err(output_err)
    }
}

/// In-memory sink for tests and dry runs.
#[derive(Debug, Default)]
pub struct VecSink {
    pub events: Vec<Event>,
}

impl EventSink for VecSink {
    fn write(&mut self, event: &Event) -> Result<()> {
        self.events.push(event.clone());
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

//! The collector's state machine. Pure logic: no sockets, no clocks, no files.
//!
//! Redundancy model (challenge Extension 1, SPECS §8.1 "Handle sequence gaps"):
//! - Every socket is subscribed to the same markets. Only the **active**
//!   socket changes the books. Standby sockets are tracked for `seq` health
//!   and nothing else, so they cannot corrupt the book.
//! - `seq` counts messages per socket and subscription (`sid`). A jump means
//!   a message was lost on that subscription, so every market fed by it is
//!   marked unsynced and a fresh snapshot is requested.
//! - `clock` is the exchange clock, identical on every connection. A snapshot
//!   includes every change with `clock <=` its own, so after a switch any
//!   delta from the new socket with `clock <=` the snapshot clock is a
//!   duplicate and is dropped.
//! - Deltas are applied only while a market is synced. A market becomes
//!   unsynced on activation, on a sequence gap, on an out-of-order clock, or
//!   on an inconsistent level, and it resyncs on the next snapshot.

use std::collections::{BTreeMap, HashMap};

use crate::book::{Book, LevelUpdate};
use crate::kalshi::WireMessage;

pub type SocketId = u32;

/// Full book for one market, in fixed point. Bids best-first, asks best-first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSnapshot {
    pub symbol: String,
    pub received_at_us: i64,
    pub exchange_ts_us: i64,
    pub bids: Vec<(i64, i64)>,
    pub asks: Vec<(i64, i64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Snapshot(BookSnapshot),
    Level {
        received_at_us: i64,
        exchange_ts_us: i64,
        update: LevelUpdate,
    },
}

/// Something the caller must do on the wire, such as calling `get_snapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    RequestSnapshot { socket: SocketId, symbol: String },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub events: Vec<Event>,
    pub commands: Vec<Command>,
}

#[derive(Debug)]
struct Market {
    book: Book,
    synced: bool,
    /// After a snapshot, drop deltas whose clock is at or below this value.
    dedupe_through: Option<u64>,
    /// The (socket, sid) subscription that produced the current book.
    feed: Option<(SocketId, u64)>,
    last_snapshot_us: i64,
    last_exchange_ts_us: i64,
}

impl Market {
    fn new(symbol: &str) -> Self {
        Self {
            book: Book::new(symbol),
            synced: false,
            dedupe_through: None,
            feed: None,
            last_snapshot_us: 0,
            last_exchange_ts_us: 0,
        }
    }
}

#[derive(Debug)]
pub struct Engine {
    active: Option<SocketId>,
    last_seq: HashMap<(SocketId, u64), u64>,
    markets: BTreeMap<String, Market>,
    snapshot_interval_us: i64,
}

fn us_from_ms(ms: i64) -> i64 {
    ms.saturating_mul(1_000)
}

impl Engine {
    pub fn new(snapshot_interval_us: i64) -> Self {
        Self {
            active: None,
            last_seq: HashMap::new(),
            markets: BTreeMap::new(),
            snapshot_interval_us,
        }
    }

    pub fn active_socket(&self) -> Option<SocketId> {
        self.active
    }

    pub fn is_synced(&self, symbol: &str) -> bool {
        self.markets.get(symbol).is_some_and(|m| m.synced)
    }

    pub fn book(&self, symbol: &str) -> Option<&Book> {
        self.markets.get(symbol).map(|m| &m.book)
    }

    /// Make `socket` the source of truth. Every known market is unsynced and a
    /// snapshot is requested from the new socket before any of its deltas count.
    pub fn activate(&mut self, socket: SocketId) -> Output {
        self.active = Some(socket);
        let mut out = Output::default();
        for (symbol, market) in self.markets.iter_mut() {
            market.synced = false;
            market.feed = None;
            out.commands.push(Command::RequestSnapshot {
                socket,
                symbol: symbol.clone(),
            });
        }
        out
    }

    /// Stop tracking a market (for example, after it stops trading).
    pub fn forget(&mut self, symbol: &str) {
        self.markets.remove(symbol);
    }

    pub fn on_message(
        &mut self,
        socket: SocketId,
        received_at_us: i64,
        message: WireMessage,
    ) -> Output {
        let mut out = Output::default();
        let (sid, seq) = match &message {
            WireMessage::OrderbookSnapshot { sid, seq, .. }
            | WireMessage::OrderbookDelta { sid, seq, .. } => (*sid, *seq),
            WireMessage::Other => return out,
        };

        let key = (socket, sid);
        if let Some(&last) = self.last_seq.get(&key) {
            if seq <= last {
                // Replayed or duplicate message on this subscription.
                return out;
            }
            if seq != last + 1 {
                self.last_seq.insert(key, seq);
                self.on_gap(socket, sid, &mut out);
            }
        }
        self.last_seq.insert(key, seq);

        if Some(socket) != self.active {
            return out;
        }

        match message {
            WireMessage::OrderbookSnapshot {
                sid,
                msg,
                sending_ts_ms,
                ..
            } => {
                let exchange_ts_us = sending_ts_ms
                    .map(us_from_ms)
                    .filter(|ts| *ts > 0)
                    .unwrap_or(received_at_us);
                match Book::from_snapshot(&msg) {
                    Ok(book) => {
                        let market = self
                            .markets
                            .entry(msg.market_ticker.clone())
                            .or_insert_with(|| Market::new(&msg.market_ticker));
                        market.book = book;
                        market.synced = true;
                        market.dedupe_through = Some(msg.clock);
                        market.feed = Some((socket, sid));
                        market.last_snapshot_us = received_at_us;
                        market.last_exchange_ts_us = exchange_ts_us;
                        out.events.push(Event::Snapshot(snapshot_of(
                            &market.book,
                            received_at_us,
                            exchange_ts_us,
                        )));
                    }
                    Err(_) => {
                        self.mark_unsynced(&msg.market_ticker);
                        out.commands.push(Command::RequestSnapshot {
                            socket,
                            symbol: msg.market_ticker,
                        });
                    }
                }
            }
            WireMessage::OrderbookDelta {
                msg, sending_ts_ms, ..
            } => {
                let is_new = !self.markets.contains_key(&msg.market_ticker);
                let market = self
                    .markets
                    .entry(msg.market_ticker.clone())
                    .or_insert_with(|| Market::new(&msg.market_ticker));

                if !market.synced {
                    // Dropped until a snapshot arrives. A known unsynced market
                    // already has a request in flight; a new one needs one now.
                    if is_new {
                        out.commands.push(Command::RequestSnapshot {
                            socket,
                            symbol: msg.market_ticker.clone(),
                        });
                    }
                    return out;
                }

                if let Some(through) = market.dedupe_through {
                    if msg.clock <= through {
                        return out;
                    }
                    market.dedupe_through = None;
                }

                if msg.clock < market.book.clock {
                    // The exchange clock went backwards on this market.
                    market.synced = false;
                    out.commands.push(Command::RequestSnapshot {
                        socket,
                        symbol: msg.market_ticker.clone(),
                    });
                    return out;
                }

                let exchange_ts_us = msg
                    .ts_ms
                    .or(sending_ts_ms)
                    .map(us_from_ms)
                    .filter(|ts| *ts > 0)
                    .unwrap_or(received_at_us);

                match market.book.apply_delta(&msg) {
                    Ok(update) => {
                        market.last_exchange_ts_us = exchange_ts_us;
                        out.events.push(Event::Level {
                            received_at_us,
                            exchange_ts_us,
                            update,
                        });
                    }
                    Err(_) => {
                        market.synced = false;
                        out.commands.push(Command::RequestSnapshot {
                            socket,
                            symbol: msg.market_ticker.clone(),
                        });
                    }
                }
            }
            WireMessage::Other => {}
        }

        out
    }

    /// Emit a full snapshot for every synced market whose last snapshot is
    /// older than the configured interval.
    pub fn on_tick(&mut self, now_us: i64) -> Output {
        let mut out = Output::default();
        for market in self.markets.values_mut() {
            if !market.synced {
                continue;
            }
            if now_us - market.last_snapshot_us < self.snapshot_interval_us {
                continue;
            }
            market.last_snapshot_us = now_us;
            out.events.push(Event::Snapshot(snapshot_of(
                &market.book,
                now_us,
                market.last_exchange_ts_us,
            )));
        }
        out
    }

    fn mark_unsynced(&mut self, symbol: &str) {
        if let Some(market) = self.markets.get_mut(symbol) {
            market.synced = false;
        }
    }

    fn on_gap(&mut self, socket: SocketId, sid: u64, out: &mut Output) {
        if Some(socket) != self.active {
            return;
        }
        for (symbol, market) in self.markets.iter_mut() {
            if market.feed == Some((socket, sid)) && market.synced {
                market.synced = false;
                out.commands.push(Command::RequestSnapshot {
                    socket,
                    symbol: symbol.clone(),
                });
            }
        }
    }
}

fn snapshot_of(book: &Book, received_at_us: i64, exchange_ts_us: i64) -> BookSnapshot {
    BookSnapshot {
        symbol: book.symbol.clone(),
        received_at_us,
        exchange_ts_us,
        bids: book.bids.iter().rev().map(|(p, q)| (*p, *q)).collect(),
        asks: book.asks.iter().map(|(p, q)| (*p, *q)).collect(),
    }
}

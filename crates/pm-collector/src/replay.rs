//! Replays a recorded session through the engine, deterministically.
//!
//! Input is JSON Lines. Each line is one of:
//! - `{"kind":"message","at_us":..,"socket":1,"payload":{..Kalshi message..}}`
//! - `{"kind":"activate","at_us":..,"socket":2}`  (failover to a socket)
//! - `{"kind":"tick","at_us":..}`                  (clock tick, for minute snapshots)
//!
//! The live collector will write this same format as its raw store, so a
//! recorded run can be replayed to reproduce the exact output.

use std::io::BufRead;

use serde::Deserialize;

use crate::engine::{Command, Engine, Output, SocketId};
use crate::error::{CollectorError, Result};
use crate::kalshi::WireMessage;
use crate::output::EventSink;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogRecord {
    Message {
        at_us: i64,
        socket: SocketId,
        payload: WireMessage,
    },
    Activate {
        at_us: i64,
        socket: SocketId,
    },
    Tick {
        at_us: i64,
    },
}

/// Run a recorded session. Returns every command the engine asked for, since
/// a replay cannot perform them.
pub fn replay(
    input: impl BufRead,
    engine: &mut Engine,
    sink: &mut dyn EventSink,
) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: LogRecord = serde_json::from_str(&line)
            .map_err(|e| CollectorError::Parse(format!("line {}: {e}", index + 1)))?;
        let out: Output = match record {
            LogRecord::Message {
                at_us,
                socket,
                payload,
            } => engine.on_message(socket, at_us, payload),
            LogRecord::Activate { socket, .. } => engine.activate(socket),
            LogRecord::Tick { at_us } => engine.on_tick(at_us),
        };
        for event in &out.events {
            sink.write(event)?;
        }
        commands.extend(out.commands);
    }
    sink.finish()?;
    Ok(commands)
}

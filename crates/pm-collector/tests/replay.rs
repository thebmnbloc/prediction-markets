//! End-to-end: a recorded Kalshi session, replayed through the engine, checked
//! against the expected books, events, and resync requests.

use std::io::Cursor;

use engineering_challenge::util::dtos::challenge_orderbook::AnyMessage;
use engineering_challenge::util::{AlignedSbeReader, OrderbookSchema};
use pm_collector::engine::{BookSnapshot, Command, Engine, Event};
use pm_collector::output::{SbeSink, VecSink};
use pm_collector::replay::replay;

const SESSION: &str = include_str!("fixtures/kalshi_session.jsonl");
const MINUTE_US: i64 = 60_000_000;

fn run() -> (Engine, VecSink, Vec<Command>) {
    let mut engine = Engine::new(MINUTE_US);
    let mut sink = VecSink::default();
    let commands = replay(Cursor::new(SESSION), &mut engine, &mut sink).unwrap();
    (engine, sink, commands)
}

#[test]
fn emits_expected_events_in_order() {
    let (_, sink, _) = run();
    let kinds: Vec<String> = sink
        .events
        .iter()
        .map(|e| match e {
            Event::Snapshot(_) => "snapshot".to_string(),
            Event::Level { update, .. } => format!(
                "level {:?} {} {}",
                update.side, update.price, update.quantity
            ),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "snapshot",
            "level Bid 400000 60000000",
            "level Bid 420000 10000000",
            "snapshot",
            "level Bid 420000 0",
            "snapshot",
            "level Bid 400000 62000000",
            "level Ask 460000 15000000",
            "snapshot",
        ]
    );
}

#[test]
fn sequence_gap_requests_resync_and_drops_until_snapshot() {
    let (_, _, commands) = run();
    let resync_for_gap = commands
        .iter()
        .filter(|c| matches!(c, Command::RequestSnapshot { socket: 1, .. }))
        .count();
    // One request from the seq gap on socket 1; one from activating socket 2.
    assert_eq!(commands.len(), 2);
    assert_eq!(resync_for_gap, 1);
    assert_eq!(
        commands[1],
        Command::RequestSnapshot {
            socket: 2,
            symbol: "KXTEST-A".into()
        }
    );
}

#[test]
fn snapshot_dedupes_deltas_at_or_below_its_clock() {
    let (_, sink, _) = run();
    // The delta on seq 8 has clock 104, equal to the resync snapshot's clock.
    // It must not appear as a level, since the snapshot already contains it.
    // The first level after the resync snapshot must be the seq 9 removal.
    let snapshot_positions: Vec<usize> = sink
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::Snapshot(_)))
        .map(|(i, _)| i)
        .collect();
    let after_resync = &sink.events[snapshot_positions[1] + 1];
    assert!(matches!(after_resync, Event::Level { update, .. } if update.quantity == 0));
}

#[test]
fn standby_socket_never_changes_the_book() {
    let (engine, _, _) = run();
    // Socket 1 sent a delta at price 0.4000 while socket 2 was active. The
    // final bid must match socket 2's view: 60 + 2 = 62, not 63.
    let book = engine.book("KXTEST-A").unwrap();
    assert_eq!(book.bids.get(&400_000), Some(&62_000_000));
}

#[test]
fn final_book_matches_the_recorded_state() {
    let (engine, _, _) = run();
    let book = engine.book("KXTEST-A").unwrap();
    assert_eq!(book.best_bid(), Some(400_000));
    assert_eq!(book.best_ask(), Some(460_000));
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.get(&460_000), Some(&15_000_000));
    assert_eq!(book.clock, 106);
    assert!(engine.is_synced("KXTEST-A"));
}

#[test]
fn snapshot_event_carries_full_book_best_first() {
    let (_, sink, _) = run();
    let Event::Snapshot(BookSnapshot { bids, asks, .. }) = &sink.events[sink.events.len() - 1]
    else {
        panic!("last event should be the minute snapshot");
    };
    assert_eq!(bids, &vec![(400_000, 62_000_000)]);
    assert_eq!(asks, &vec![(460_000, 15_000_000)]);
}

#[test]
fn sbe_output_round_trips_through_the_challenge_reader() {
    let path = std::env::temp_dir().join(format!("pm-collector-{}.sbe", std::process::id()));
    let mut engine = Engine::new(MINUTE_US);
    let mut sink = SbeSink::create(&path).unwrap();
    replay(Cursor::new(SESSION), &mut engine, &mut sink).unwrap();

    let mut reader = AlignedSbeReader::<OrderbookSchema>::open(&path).unwrap();
    let mut snapshots = 0;
    let mut levels = 0;
    while let Some(message) = reader.read_msg().unwrap() {
        match message {
            AnyMessage::Snapshot(_) => snapshots += 1,
            AnyMessage::PriceLevelUpdate(_) => levels += 1,
            _ => panic!("unexpected message type"),
        }
    }
    std::fs::remove_file(&path).ok();

    assert_eq!(snapshots, 4);
    assert_eq!(levels, 5);
}

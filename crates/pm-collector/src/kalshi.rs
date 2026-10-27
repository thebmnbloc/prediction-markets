//! Wire format for Kalshi's `orderbook_snapshot` and `orderbook_delta`
//! messages. Parsing only: no network, no state.
//! Source: https://docs.kalshi.com/websockets/orderbook-updates

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WireMessage {
    OrderbookSnapshot {
        sid: u64,
        seq: u64,
        msg: SnapshotBody,
        #[serde(default)]
        sending_ts_ms: Option<i64>,
    },
    OrderbookDelta {
        sid: u64,
        seq: u64,
        msg: DeltaBody,
        #[serde(default)]
        sending_ts_ms: Option<i64>,
    },
    /// Subscription acks, errors, and any type this collector does not use.
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SnapshotBody {
    pub market_ticker: String,
    #[serde(default)]
    pub market_id: Option<String>,
    pub clock: u64,
    /// `[price_in_dollars, contract_count_fp]` pairs for YES bids.
    #[serde(default)]
    pub yes_dollars_fp: Vec<[String; 2]>,
    /// `[price_in_dollars, contract_count_fp]` pairs for NO bids.
    #[serde(default)]
    pub no_dollars_fp: Vec<[String; 2]>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DeltaBody {
    pub market_ticker: String,
    #[serde(default)]
    pub market_id: Option<String>,
    pub clock: u64,
    pub price_dollars: String,
    /// Signed change in contracts at this level, not an absolute quantity.
    pub delta_fp: String,
    pub side: WireSide,
    #[serde(default)]
    pub ts_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireSide {
    Yes,
    No,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_snapshot() {
        let text = r#"{
            "type": "orderbook_snapshot", "sending_ts_ms": 1669149841234,
            "sid": 2, "seq": 2,
            "msg": {"market_ticker": "FED-23DEC-T3.00",
                    "market_id": "9b0f6b43-5b68-4f9f-9f02-9a2d1b8ac1a1",
                    "clock": 4182057,
                    "yes_dollars_fp": [["0.0800","300.00"],["0.2200","333.00"]],
                    "no_dollars_fp": [["0.5400","20.00"],["0.5600","146.00"]]}
        }"#;
        match serde_json::from_str::<WireMessage>(text).unwrap() {
            WireMessage::OrderbookSnapshot { sid, seq, msg, .. } => {
                assert_eq!(sid, 2);
                assert_eq!(seq, 2);
                assert_eq!(msg.clock, 4182057);
                assert_eq!(msg.yes_dollars_fp[1][0], "0.2200");
                assert_eq!(msg.no_dollars_fp.len(), 2);
            }
            other => panic!("expected snapshot, got {other:?}"),
        }
    }

    #[test]
    fn parses_documented_delta() {
        let text = r#"{
            "type": "orderbook_delta", "sending_ts_ms": 1669149841123,
            "sid": 2, "seq": 3,
            "msg": {"market_ticker": "FED-23DEC-T3.00",
                    "market_id": "9b0f6b43-5b68-4f9f-9f02-9a2d1b8ac1a1",
                    "clock": 4182061, "price_dollars": "0.9600",
                    "delta_fp": "-54.00", "side": "yes",
                    "ts": "2022-11-22T20:44:01Z", "ts_ms": 1669149841000}
        }"#;
        match serde_json::from_str::<WireMessage>(text).unwrap() {
            WireMessage::OrderbookDelta { msg, .. } => {
                assert_eq!(msg.delta_fp, "-54.00");
                assert_eq!(msg.side, WireSide::Yes);
                assert_eq!(msg.ts_ms, Some(1669149841000));
            }
            other => panic!("expected delta, got {other:?}"),
        }
    }

    #[test]
    fn unknown_types_are_ignored_not_errors() {
        let text = r#"{"type": "subscribed", "id": 1, "msg": {"channel": "orderbook_delta"}}"#;
        assert_eq!(
            serde_json::from_str::<WireMessage>(text).unwrap(),
            WireMessage::Other
        );
    }
}

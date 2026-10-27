# Prediction-Market Quant System --- Technical Specification

**Document:** `SPECS.md`\
**Status:** Draft for implementation\
**Version:** 0.2\
**Scope:** Research, market-data ingestion, opportunity discovery, trade
scoring, risk controls, backtesting, and optional human-approved
execution for Kalshi and Polymarket.

------------------------------------------------------------------------

## 1. Purpose and product requirements

Build a system that continuously discovers, evaluates, ranks, and
explains prediction-market trading opportunities. The first release is a
**decision-support system**: it produces ranked candidates and evidence;
a human approves any live order. Automated execution is explicitly out
of scope for the MVP and must remain disabled until the strategy has
passed validation and operational risk reviews.

### 1.1 Primary capabilities

1.  Ingest markets, order books, trades, market metadata, fees,
    settlement rules, and market status from supported venues.
2.  Normalize venue-specific data into a common internal schema without
    losing the original venue payload.
3.  Preserve point-in-time snapshots and provenance for reproducible
    research and backtesting.
4.  Estimate fair probabilities using transparent baseline models and
    optional event-specific models.
5.  Detect:
    -   probability-value opportunities;
    -   cross-venue price discrepancies;
    -   logically related-market inconsistencies;
    -   event/news-driven changes;
    -   execution-quality and liquidity changes.
6.  Calculate net expected value, uncertainty-adjusted edge, execution
    costs, liquidity limits, and risk flags.
7.  Rank opportunities and explain why each was surfaced.
8.  Provide a dashboard/API for filtering, inspecting, paper-trading,
    and reviewing outcomes.
9.  Measure model calibration and strategy performance using
    leakage-safe backtests.
10. Fail safely when data are stale, incomplete, contradictory, or
    unavailable.

### 1.2 Non-goals for the MVP

-   Unsupervised autonomous trading or unattended fund movement.
-   Guaranteed-profit claims.
-   Treating every difference between displayed prices as arbitrage.
-   Automatically interpreting arbitrary market settlement language
    without review.
-   High-frequency market making.
-   Using an LLM output as the sole probability estimate or order
    authorization.
-   Circumventing venue rules, geographic restrictions, sanctions, or
    account eligibility requirements.

### 1.3 MVP acceptance criteria

-   Both venue adapters can be independently enabled/disabled.
-   Every stored observation has a venue, market identifier, observation
    timestamp, ingestion timestamp, and source/provenance.
-   The system can reconstruct the information available at a historical
    decision time.
-   Every candidate includes executable-side price inputs, estimated
    costs, model version, probability estimate, net EV,
    confidence/uncertainty, liquidity checks, and a human-readable
    rationale.
-   Stale or incomplete data cannot create an actionable candidate.
-   Backtests account for fees, spread, slippage, partial fills where
    modeled, market resolution, and delisted/cancelled markets.
-   Paper-trading results can be compared against a baseline and
    exported.
-   Live execution is off by default and cannot be enabled solely by
    changing a UI field.

------------------------------------------------------------------------

## 2. System architecture

### 2.1 Logical components

``` text
Kalshi Adapter ─┐
                ├─> Ingestion Queue ─> Normalizer ─> Raw Store + Canonical Store
Polymarket Adapter┘                                  │
                                                     ├─> Market Snapshot Service
External Data / News ─> Source Adapters ─────────────┤
                                                     ├─> Feature Builder
                                                     │      └─> Probability Models
                                                     │
                                                     └─> Opportunity Engine
                                                            ├─> Value Signals
                                                            ├─> Cross-Market Checks
                                                            ├─> Cost / Liquidity Model
                                                            ├─> Risk Gate
                                                            └─> Ranked Candidates
                                                                    │
                                                    API / Dashboard / Alerts
                                                                    │
                                                    Human Approval (later)
                                                                    │
                                                    Order Manager (disabled MVP)
```

### 2.2 Recommended initial stack

-   **Backend language:** Rust stable for ingestion, normalization,
    scoring, risk checks, API, and paper trading. Pin a tested toolchain
    in `rust-toolchain.toml`.
-   **HTTP API:** Axum + Tokio, with Tower middleware for timeouts,
    tracing, request IDs, and request limits.
-   **Serialization:** Serde + serde_json; validate domain invariants in
    explicit constructors and typed domain models.
-   **Market I/O:** reqwest for REST and tokio-tungstenite (or a
    maintained venue client) for WebSockets.
-   **Database:** PostgreSQL 16+ with SQLx and versioned SQLx
    migrations.
-   **Money / probabilities:** rust_decimal for money, contract prices,
    fees, sizes, and P&L; explicit bounded conversions for statistical
    calculations.
-   **Research storage:** Parquet for bulk historical snapshots and
    experiment outputs.
-   **Queue:** Tokio tasks and bounded channels initially; Redis Streams
    is optional when durable queueing is needed. Add Kafka/Redpanda only
    when throughput warrants it.
-   **Frontend:** Next.js + TypeScript for the dashboard, with a typed
    client for the Rust API.
-   **API contract:** OpenAPI generated/documented from the Rust service
    (for example, utoipa) and consumed by a generated or maintained
    TypeScript client.
-   **Cache:** Redis optional for hot books and latest snapshots.
-   **Deployment:** Docker Compose locally; a single VM is sufficient
    for early development.
-   **Observability:** tracing + tracing-subscriber,
    Prometheus-compatible metrics, health endpoints, and error
    reporting.
-   **Testing:** cargo test, fixture-based contract tests, property
    tests for financial calculations, and frontend lint/type-check/build
    tests.

### 2.3 Service boundaries

1.  `adapters`: venue-specific Rust API clients and data collectors.
2.  `normalizer`: canonical market, outcome, book, trade, fee, and
    status conversion.
3.  `storage`: PostgreSQL repositories and point-in-time query
    interfaces.
4.  `features`: feature computation and external-source provenance.
5.  `models`: probability models, calibration, and model registry
    metadata.
6.  `signals`: value, arbitrage, and related-market detectors.
7.  `risk`: exposure checks and candidate rejection rules.
8.  `api`: Axum routes for read-only MVP endpoints and paper-trading
    operations.
9.  `frontend`: Next.js dashboard for ranked opportunities and candidate
    detail pages.
10. `paper`: simulated order lifecycle and settlement.
11. `execution`: interface definitions only in MVP; live implementation
    disabled.

## 3. Core design principles

-   **Executable prices, not midpoints:** a buy signal uses the ask and
    a sell signal uses the bid, adjusted for expected fill price.
-   **No hidden assumptions:** every estimate must identify its model,
    input snapshot, fee schedule, and assumptions.
-   **Point-in-time correctness:** features must use only data available
    by the decision timestamp.
-   **Uncertainty matters:** probability estimates must be calibrated
    and accompanied by uncertainty or conservative bounds.
-   **Settlement semantics matter:** contracts are comparable only when
    the outcome, resolution source, deadline, cancellation rules, and
    payout semantics are compatible.
-   **Risk gates override ranking:** a high score never bypasses
    stale-data, liquidity, eligibility, or exposure checks.
-   **Reproducibility:** persist model/config versions, feature
    versions, and input identifiers for each signal.
-   **Safe default:** read-only / paper mode by default; no private keys
    in source control or logs.
-   **Venue terms and applicable law:** validate account eligibility,
    API usage, geographic restrictions, and tax/regulatory obligations
    before live use.

------------------------------------------------------------------------

## 4. Canonical data model

Use UUIDs for internal primary keys. Use `NUMERIC`/decimal arithmetic
for money and contract prices; do not use binary floating point for
order sizing or settlement cash calculations. Timestamps are UTC
(`TIMESTAMPTZ`). Preserve raw payloads as JSONB or immutable
object-store records.

### 4.1 `venues`

  Field            Type          Constraints / meaning
  ---------------- ------------- --------------------------------
  `id`             UUID          Primary key
  `code`           TEXT          Unique: `kalshi`, `polymarket`
  `display_name`   TEXT          Human-readable
  `enabled`        BOOLEAN       Adapter enabled
  `created_at`     TIMESTAMPTZ   Creation time
  `updated_at`     TIMESTAMPTZ   Last update

### 4.2 `events`

An event groups one or more markets about the same real-world question
or occurrence. Cross-venue matching must not assume that similar titles
imply equivalent settlement semantics.

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `canonical_key`         TEXT                    Nullable; normalized
                                                  internal identity

  `title`                 TEXT                    Canonical event title

  `category`              TEXT                    Politics, economics,
                                                  sports, crypto,
                                                  weather, other

  `description`           TEXT                    Nullable

  `event_start_at`        TIMESTAMPTZ             Nullable

  `event_end_at`          TIMESTAMPTZ             Nullable

  `source_refs`           JSONB                   Official source URLs
                                                  and metadata

  `created_at`            TIMESTAMPTZ             Creation time
  -----------------------------------------------------------------------

### 4.3 `markets`

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `venue_id`              UUID                    FK → `venues.id`

  `venue_market_id`       TEXT                    Venue-native
                                                  market/contract ID

  `event_id`              UUID                    Nullable FK →
                                                  `events.id`

  `title`                 TEXT                    Original title

  `rules_text`            TEXT                    Full available
                                                  settlement rules

  `rules_hash`            TEXT                    Hash of rules version

  `category`              TEXT                    Normalized category

  `market_type`           TEXT                    `binary`,
                                                  `categorical`,
                                                  `scalar`, `unknown`

  `status`                TEXT                    `open`, `closed`,
                                                  `suspended`,
                                                  `resolved`,
                                                  `cancelled`, `unknown`

  `opens_at`              TIMESTAMPTZ             Nullable

  `closes_at`             TIMESTAMPTZ             Nullable

  `resolves_at`           TIMESTAMPTZ             Nullable

  `resolution_source`     TEXT                    Nullable; authoritative
                                                  source/URL

  `settlement_spec`       JSONB                   Structured
                                                  resolution/payout
                                                  semantics

  `currency`              TEXT                    Currency/settlement
                                                  unit

  `tick_size`             NUMERIC                 Nullable

  `min_order_size`        NUMERIC                 Nullable

  `fee_schedule_id`       UUID                    Nullable FK

  `last_seen_at`          TIMESTAMPTZ             Last confirmed from
                                                  venue

  `created_at`            TIMESTAMPTZ             First stored

  `updated_at`            TIMESTAMPTZ             Last metadata update
  -----------------------------------------------------------------------

Unique constraint: `(venue_id, venue_market_id)`. Keep historical rules
changes in a separate `market_rule_versions` table or append-only
metadata history; do not overwrite rules in a way that destroys
historical meaning.

### 4.4 `outcomes`

Each market has one or more outcomes. For binary markets, use canonical
outcomes such as `YES` and `NO`, while retaining venue-native labels.

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `market_id`             UUID                    FK → `markets.id`

  `venue_outcome_id`      TEXT                    Nullable venue-native
                                                  outcome/token ID

  `outcome_key`           TEXT                    Canonical key

  `label`                 TEXT                    Venue-native label

  `payout_if_wins`        NUMERIC                 Per-contract payout in
                                                  settlement units

  `payout_if_loses`       NUMERIC                 Usually zero; model
                                                  explicitly

  `is_complement`         BOOLEAN                 Whether this is the
                                                  complement of another
                                                  outcome

  `metadata`              JSONB                   Additional venue fields
  -----------------------------------------------------------------------

Unique constraint: `(market_id, outcome_key)` where applicable. Do not
assume every market has exactly two outcomes.

### 4.5 `order_book_snapshots`

Store normalized levels and the raw venue payload. Snapshot cadence
should be configurable per venue and market liquidity.

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `venue_id`              UUID                    FK

  `market_id`             UUID                    FK

  `outcome_id`            UUID                    FK

  `observed_at`           TIMESTAMPTZ             Venue timestamp if
                                                  supplied

  `received_at`           TIMESTAMPTZ             Local receive timestamp

  `sequence_no`           TEXT                    Nullable venue sequence

  `best_bid`              NUMERIC                 Nullable

  `best_ask`              NUMERIC                 Nullable

  `bid_depth`             NUMERIC                 Depth within configured
                                                  price band

  `ask_depth`             NUMERIC                 Depth within configured
                                                  price band

  `spread`                NUMERIC                 `best_ask - best_bid`
                                                  when both valid

  `levels`                JSONB                   `[{price, size}]` for
                                                  bids and asks

  `is_complete`           BOOLEAN                 False if snapshot is
                                                  partial or sequence gap
                                                  detected

  `raw_payload_ref`       TEXT                    Nullable raw object
                                                  reference

  `ingested_at`           TIMESTAMPTZ             Persistence time
  -----------------------------------------------------------------------

Indexes: `(market_id, outcome_id, observed_at DESC)`,
`(venue_id, received_at DESC)`. Retention policy should preserve enough
history for evaluation and audit.

### 4.6 `trades`

  Field              Type          Constraints / meaning
  ------------------ ------------- --------------------------
  `id`               UUID          Primary key
  `venue_id`         UUID          FK
  `market_id`        UUID          FK
  `outcome_id`       UUID          FK
  `venue_trade_id`   TEXT          Nullable native ID
  `price`            NUMERIC       Trade price
  `size`             NUMERIC       Trade size
  `aggressor_side`   TEXT          `buy`, `sell`, `unknown`
  `executed_at`      TIMESTAMPTZ   Venue timestamp
  `received_at`      TIMESTAMPTZ   Local receive timestamp
  `raw_payload`      JSONB         Optional payload

Use a unique key on `(venue_id, venue_trade_id)` when the venue supplies
stable IDs.

### 4.7 `fee_schedules`

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `venue_id`              UUID                    FK

  `effective_from`        TIMESTAMPTZ             Start time

  `effective_to`          TIMESTAMPTZ             Nullable end time

  `fee_model`             TEXT                    `per_contract`,
                                                  `percentage`, `tiered`,
                                                  `unknown`

  `parameters`            JSONB                   Versioned fee
                                                  calculation parameters

  `source_url`            TEXT                    Official source

  `verified_at`           TIMESTAMPTZ             Last verified time
  -----------------------------------------------------------------------

Fees must be computed from a versioned, venue-specific calculator. Never
assume one universal fee formula or that a fee schedule stays constant.

### 4.8 `external_observations`

  -----------------------------------------------------------------------
  Field                   Type                    Constraints / meaning
  ----------------------- ----------------------- -----------------------
  `id`                    UUID                    Primary key

  `source`                TEXT                    Provider/source name

  `source_record_id`      TEXT                    Nullable provider ID

  `event_id`              UUID                    Nullable FK

  `observed_at`           TIMESTAMPTZ             When the fact/event
                                                  occurred

  `published_at`          TIMESTAMPTZ             When published, if
                                                  known

  `ingested_at`           TIMESTAMPTZ             When our system
                                                  received it

  `kind`                  TEXT                    `news`, `poll`,
                                                  `economic_release`,
                                                  `weather`,
                                                  `official_result`,
                                                  `other`

  `value_numeric`         NUMERIC                 Nullable

  `value_text`            TEXT                    Nullable

  `payload`               JSONB                   Normalized source data

  `source_url`            TEXT                    Provenance

  `quality_score`         NUMERIC                 Optional, documented
                                                  scale
  -----------------------------------------------------------------------

Do not treat publication time as observation time. Backtests must only
expose records that were actually available by the simulated decision
time.

### 4.9 `probability_estimates`

  Field                   Type          Constraints / meaning
  ----------------------- ------------- -------------------------------------------
  `id`                    UUID          Primary key
  `market_id`             UUID          FK
  `outcome_id`            UUID          FK
  `as_of`                 TIMESTAMPTZ   Decision time
  `p_raw`                 NUMERIC       Raw model probability, in \[0,1\]
  `p_calibrated`          NUMERIC       Calibrated probability, in \[0,1\]
  `p_lower`               NUMERIC       Conservative lower bound, optional
  `p_upper`               NUMERIC       Conservative upper bound, optional
  `model_name`            TEXT          Model identity
  `model_version`         TEXT          Immutable model version
  `feature_set_version`   TEXT          Feature version
  `input_snapshot_ids`    JSONB         IDs/refs of exact inputs
  `explanation`           JSONB         Features and evidence supporting estimate
  `created_at`            TIMESTAMPTZ   Persisted time

For categorical markets, estimates for mutually exclusive exhaustive
outcomes should sum to approximately 1, subject to an explicit
normalization policy. Binary YES/NO estimates should be complementary
unless the market rules create a nonstandard outcome structure.

### 4.10 `signals`

  -------------------------------------------------------------------------
  Field                       Type                 Constraints / meaning
  --------------------------- -------------------- ------------------------
  `id`                        UUID                 Primary key

  `market_id`                 UUID                 FK

  `outcome_id`                UUID                 FK

  `signal_type`               TEXT                 `value`, `cross_venue`,
                                                   `logical_consistency`,
                                                   `event_driven`

  `side`                      TEXT                 `buy`, `sell`, `paired`,
                                                   `watch`

  `decision_at`               TIMESTAMPTZ          Signal calculation time

  `reference_price`           NUMERIC              Price used in
                                                   calculation

  `executable_price`          NUMERIC              Estimated executable
                                                   price

  `p_estimate`                NUMERIC              Model probability

  `gross_ev_per_contract`     NUMERIC              Before fees/slippage

  `net_ev_per_contract`       NUMERIC              After estimated costs

  `conservative_net_ev`       NUMERIC              EV using conservative
                                                   probability/price

  `expected_return_on_cost`   NUMERIC              Nullable if denominator
                                                   invalid

  `uncertainty`               NUMERIC              Defined model-specific
                                                   uncertainty metric

  `score`                     NUMERIC              Versioned ranking score

  `status`                    TEXT                 `candidate`, `rejected`,
                                                   `watch`, `approved`,
                                                   `expired`

  `reason_codes`              JSONB                Machine-readable
                                                   reasons/flags

  `rationale`                 JSONB                Human-readable evidence

  `model_version`             TEXT                 Model version

  `scoring_version`           TEXT                 Scoring algorithm
                                                   version

  `expires_at`                TIMESTAMPTZ          Candidate TTL

  `created_at`                TIMESTAMPTZ          Persisted time
  -------------------------------------------------------------------------

### 4.11 `cross_market_links`

Used for manually reviewed or algorithmically proposed relationships.

  -----------------------------------------------------------------------
  Field                     Type                  Constraints / meaning
  ------------------------- --------------------- -----------------------
  `id`                      UUID                  Primary key

  `market_a_id`             UUID                  FK

  `outcome_a_id`            UUID                  FK

  `market_b_id`             UUID                  FK

  `outcome_b_id`            UUID                  FK

  `relation_type`           TEXT                  `equivalent`,
                                                  `complement`,
                                                  `implies`,
                                                  `mutually_exclusive`,
                                                  `correlated`, `unknown`

  `semantic_match_score`    NUMERIC               Candidate similarity
                                                  score, not proof

  `settlement_compatible`   BOOLEAN               Human-verified
                                                  compatibility

  `review_status`           TEXT                  `unreviewed`,
                                                  `approved`, `rejected`

  `review_notes`            TEXT                  Nullable

  `created_at`              TIMESTAMPTZ           Creation time
  -----------------------------------------------------------------------

Only `approved` links with compatible settlement rules can trigger
actionable cross-market arbitrage checks. Other relationships may create
a research alert only.

### 4.12 `paper_orders`, `paper_fills`, `positions`

Paper-trading records should be distinct from live orders.

`paper_orders` fields: - `id UUID PRIMARY KEY` -
`signal_id UUID REFERENCES signals(id)` -
`venue_id UUID REFERENCES venues(id)` -
`market_id UUID REFERENCES markets(id)` -
`outcome_id UUID REFERENCES outcomes(id)` -
`side TEXT CHECK (side IN ('buy','sell'))` - `limit_price NUMERIC` -
`requested_size NUMERIC` - `status TEXT` (`new`, `open`, `partial`,
`filled`, `cancelled`, `expired`, `rejected`) -
`submitted_at TIMESTAMPTZ` - `updated_at TIMESTAMPTZ` -
`simulation_model_version TEXT`

`paper_fills` fields: - `id UUID PRIMARY KEY` -
`paper_order_id UUID REFERENCES paper_orders(id)` -
`fill_price NUMERIC` - `fill_size NUMERIC` - `fee NUMERIC` -
`filled_at TIMESTAMPTZ` - `fill_assumption TEXT`

`positions` fields: - `id UUID PRIMARY KEY` - `venue_id UUID` -
`market_id UUID` - `outcome_id UUID` - `quantity NUMERIC` -
`average_entry_price NUMERIC` - `realized_pnl NUMERIC` -
`unrealized_pnl NUMERIC` - `updated_at TIMESTAMPTZ`

Paper fills must not assume that a limit order fills just because the
market later touched the limit. Model queue position and partial fills
conservatively, or clearly label a simplified fill assumption.

### 4.13 `model_runs` and `audit_events`

`model_runs`: - `id UUID PRIMARY KEY` - `model_name TEXT` -
`model_version TEXT` - `started_at TIMESTAMPTZ` -
`finished_at TIMESTAMPTZ` - `dataset_snapshot_ref TEXT` -
`config JSONB` - `metrics JSONB` - `status TEXT` - `code_commit TEXT`

`audit_events`: - `id UUID PRIMARY KEY` - `actor_type TEXT` (`system`,
`user`, `operator`) - `actor_id TEXT NULL` - `action TEXT` -
`entity_type TEXT` - `entity_id TEXT` - `details JSONB` -
`created_at TIMESTAMPTZ`

Never store API secrets, private keys, passwords, or authentication
tokens in audit payloads.

------------------------------------------------------------------------

## 5. Canonical API contracts

Prefix: `/api/v1`. JSON responses use ISO-8601 UTC timestamps. All
endpoints should return consistent error objects and request IDs.

### 5.1 Common error format

``` json
{
  "error": {
    "code": "STALE_MARKET_DATA",
    "message": "Market data are older than the configured maximum age.",
    "request_id": "req_..."
  }
}
```

### 5.2 Endpoints

  --------------------------------------------------------------------------
  Method             Endpoint                         Purpose
  ------------------ -------------------------------- ----------------------
  `GET`              `/health`                        Process health

  `GET`              `/ready`                         Dependency/readiness
                                                      status

  `GET`              `/venues`                        Venue adapter status

  `GET`              `/markets`                       Filter markets by
                                                      venue, status,
                                                      category, close time

  `GET`              `/markets/{market_id}`           Market metadata and
                                                      settlement rules

  `GET`              `/markets/{market_id}/book`      Latest normalized
                                                      order book

  `GET`              `/markets/{market_id}/history`   Historical
                                                      prices/trades

  `GET`              `/signals`                       Ranked signal list
                                                      with filters

  `GET`              `/signals/{signal_id}`           Full signal rationale
                                                      and inputs

  `POST`             `/signals/refresh`               Request an idempotent
                                                      signal recomputation

  `GET`              `/models`                        Active model versions
                                                      and status

  `GET`              `/metrics/strategy`              Backtest/paper metrics

  `POST`             `/paper/orders`                  Create a paper order

  `GET`              `/paper/orders`                  List paper orders

  `POST`             `/paper/orders/{id}/cancel`      Cancel a simulated
                                                      order

  `GET`              `/paper/positions`               Paper positions and
                                                      P&L

  `GET`              `/cross-market-links`            View reviewed market
                                                      relationships
  --------------------------------------------------------------------------

MVP API should be read-only except for explicit paper-trading actions.
Protect all non-public endpoints with authentication if deployed beyond
localhost.

### 5.3 Signal response example

``` json
{
  "id": "0b7a...",
  "market_id": "3c1d...",
  "outcome_id": "c9e4...",
  "venue": "polymarket",
  "title": "Example event resolves YES",
  "signal_type": "value",
  "side": "buy",
  "decision_at": "2026-10-25T12:00:00Z",
  "executable_price": "0.4200",
  "model_probability": "0.5500",
  "conservative_probability": "0.5200",
  "gross_ev_per_contract": "0.1300",
  "estimated_fee": "0.0100",
  "estimated_slippage": "0.0050",
  "net_ev_per_contract": "0.1150",
  "liquidity_ok": true,
  "data_freshness_ms": 450,
  "score": 0.73,
  "reason_codes": ["POSITIVE_CONSERVATIVE_EV", "LIQUIDITY_OK"],
  "rationale": [
    "Calibrated model probability exceeds the executable ask.",
    "Conservative probability still implies positive expected value after estimated costs."
  ],
  "model_version": "baseline-logit-v1",
  "scoring_version": "rank-v1",
  "status": "candidate",
  "expires_at": "2026-10-25T12:01:00Z"
}
```

Values above are illustrative, not live data.

------------------------------------------------------------------------

## 6. Market adapter interfaces

Each venue adapter is implemented in Rust and conforms to a shared
asynchronous trait while retaining venue-specific behavior. Keep raw
venue DTOs separate from canonical domain types; validate external
payloads before persistence.

``` rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("rate limited")]
    RateLimited,
    #[error("invalid venue payload: {0}")]
    InvalidPayload(String),
    #[error("market not found")]
    NotFound,
}

#[async_trait]
pub trait MarketAdapter: Send + Sync {
    fn venue_code(&self) -> &'static str;

    async fn healthcheck(&self) -> Result<(), AdapterError>;

    async fn list_markets(
        &self,
        cursor: Option<String>,
    ) -> Result<(Vec<Value>, Option<String>), AdapterError>;

    async fn get_market(&self, venue_market_id: &str)
        -> Result<Value, AdapterError>;

    async fn get_order_book(&self, venue_market_id: &str)
        -> Result<Value, AdapterError>;

    async fn list_trades(
        &self,
        venue_market_id: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<Value>, AdapterError>;
}
```

Adapter requirements:

-   Follow official API documentation and rate limits.
-   Use bounded retries with exponential backoff and jitter.
-   Respect `Retry-After` when supplied.
-   Handle pagination, reconnects, WebSocket heartbeats, and sequence
    gaps.
-   On a sequence gap, mark the book incomplete and request a fresh
    snapshot before using it.
-   Track source timestamps separately from local receive timestamps.
-   Avoid silently treating missing fields as zero.
-   Store raw payloads or immutable references for debugging and replay.
-   Build fixture-based contract tests for each adapter.
-   Never assume venue API semantics are identical; test normalization
    against official documentation.

## 7. Signal-generation algorithms

### 7.1 Executable price and costs

For a buy, use the ask-side book and estimate the volume-weighted price
for the intended size. For a sell, use the bid-side book. If the
requested size exceeds displayed depth, either estimate conservative
impact using a documented model or reject the candidate.

Let: - `p` = probability that the purchased outcome pays the modeled
winning payout; - `P` = expected entry cost per contract, including the
modeled fill price; - `W` = payout if the outcome wins; - `L` = payout
if it loses; - `F` = expected fees per contract; - `S` = expected
additional slippage/impact cost per contract.

Then:

`EV = p * W + (1 - p) * L - P - F - S`

For a binary contract paying `$1` on success and `$0` otherwise:

`EV = p - P - F - S`

Do not double-count slippage if `P` already reflects the estimated
volume-weighted execution price. Document exactly what each cost field
includes.

### 7.2 Conservative probability

If a model returns a calibrated probability `p` and a documented
uncertainty interval `[p_lower, p_upper]`, use the conservative bound
appropriate to the trade side: - buying YES: use a conservative lower
probability bound; - buying NO: estimate the NO probability directly or
use the complement consistently; - selling a position: calculate
expected value from the actual payout/position payoff, not by
mechanically reusing the buy formula.

If no reliable uncertainty interval exists, apply a documented
probability haircut and mark the candidate as lower confidence. Do not
present an arbitrary haircut as statistically calibrated.

### 7.3 Value signal gates

A candidate can pass only if all required conditions hold:

1.  Market is open and eligible.
2.  Market rules and payout semantics are known.
3.  Latest book is complete and younger than configured
    `MAX_BOOK_AGE_MS`.
4.  Spread and depth meet configured limits.
5.  Estimated fill price is available for the target size.
6.  Model probability is valid and the model has passed minimum
    calibration checks.
7.  Conservative net EV is greater than the minimum threshold.
8.  Exposure and concentration limits pass.
9.  Candidate has not expired and is not a duplicate of a current
    signal.
10. No unresolved data-quality or settlement-compatibility blocker
    exists.

Initial thresholds belong in configuration, not hardcoded constants.

### 7.4 Cross-venue arbitrage

A discrepancy is not necessarily an arbitrage. A candidate pair
requires: - verified equivalent or complementary settlement outcomes; -
compatible deadlines, cancellation rules, resolution sources, and payout
structures; - executable prices on both legs; - fee and slippage
estimates for both legs; - enough depth on both legs; - allowance for
legging risk, partial fills, and capital lock-up; - venue/account
eligibility and transfer/settlement constraints.

For a simplified pair where exactly one of two complementary positions
pays `W` and the combined total entry cost is `C`, the guaranteed gross
payoff is `W`; the simplified net payoff is `W - C - total_costs`,
**only if** the payoff identity and settlement compatibility have been
proven. Real cases may require a payoff matrix rather than this
shortcut.

Do not call a pair arbitrage if the relationship is only statistical
correlation. Statistical relationships should be labeled relative-value
or consistency signals and carry model risk.

### 7.5 Logical consistency checks

Represent verified relationships as constraints: - `A implies B`:
`P(A) <= P(B)`. - `A and B mutually exclusive`: `P(A) + P(B) <= 1`. -
`A and B exhaustive and mutually exclusive`: `P(A) + P(B) = 1`. -
`A is complement of B`: `P(A) + P(B) = 1`.

Prices may not equal true probabilities due to spread, fees, and risk
premia. Use executable bid/ask bounds and a tolerance, not just midpoint
comparisons. An inconsistency is a research alert until settlement
semantics and executable payoff are verified.

### 7.6 Event-driven signals

Every external observation must include source,
event/publication/ingestion times, quality, and source URL or record ID.
A news classifier or LLM may extract structured facts and link them to
candidate markets, but: - retain the source evidence; - separate
extracted claims from verified facts; - use deterministic rules or a
validated model to convert evidence into probability changes; - never
let a text-generation result place an order; - detect duplicate stories
and stale reports; - include event-specific validation and a confidence
measure.

### 7.7 Ranking score

Do not make a single score the decision-maker. Use hard risk gates
first. A versioned ranking heuristic may combine: - conservative net
EV; - expected return on capital; - model calibration/uncertainty; -
book depth and spread; - data freshness; - market resolution horizon; -
concentration/correlation penalties.

Normalize inputs before combining them. Preserve raw component values in
each signal so the score is explainable and can be recalibrated from
out-of-sample results.

------------------------------------------------------------------------

## 8. Probability models and validation

### 8.1 Model roadmap

**Phase 1 --- Baselines** - market-implied probability from executable
quotes and midpoint, stored separately; - simple historical base rates
where a defensible reference class exists; - logistic regression or
calibrated gradient boosting for one clearly defined market category; -
rule-based cross-market constraints.

**Phase 2 --- Category-specific models** - elections: polling,
fundamentals, forecast ensembles, time-to-election; - economics:
official release calendars, consensus, revisions, historical surprise
distributions; - weather: official forecast distributions and
uncertainty; - sports: team/player ratings, injuries, schedules, and
lineup uncertainty; - crypto/technology: event-specific sources and
carefully defined features.

Do not mix these into one universal model without evidence that the
pooled model is well calibrated.

### 8.2 Calibration

Evaluate probability quality using: - Brier score; - log loss; -
reliability/calibration plots; - calibration by category, horizon, and
probability bucket; - sample counts and confidence intervals; -
comparison against market-implied baseline.

Use chronological train/validation/test splits. For time-sensitive or
overlapping events, group related observations to avoid leakage.
Calibrate on a validation set, not the final test set. Avoid evaluating
only resolved markets that survived until the end; account for
cancellations and delistings.

### 8.3 Backtesting protocol

-   Use point-in-time market snapshots and external data as they were
    available at the time.
-   Create signals at historical decision timestamps; do not use future
    data.
-   Execute against realistic bid/ask and depth, not hindsight midpoint
    prices.
-   Include fees, slippage, market impact, fill uncertainty, resolution
    payout, and cancellations.
-   Separate strategy research from the final holdout test period.
-   Report performance by market category, venue, time-to-resolution,
    liquidity bucket, and signal type.
-   Compare to simple baselines, including no trade and market-implied
    probability.
-   Track turnover, exposure, drawdown, win rate, expected value,
    realized P&L, and calibration.
-   Use walk-forward validation and record the exact code/config/data
    versions.
-   Include delisted, cancelled, and unavailable markets to reduce
    survivorship bias.

A high win rate alone is not evidence of profitability; evaluate
realized net P&L and risk-adjusted performance.

------------------------------------------------------------------------

## 9. Risk management

### 9.1 MVP risk controls

-   Paper mode is the default.
-   No live order endpoint is exposed in the MVP.
-   Reject stale, incomplete, crossed, or internally inconsistent books.
-   Cap notional per market, event, category, venue, and correlated
    event group.
-   Cap total open exposure and daily loss in paper simulation.
-   Avoid duplicate orders/signals for the same thesis.
-   Track settlement and resolution risk separately from price risk.
-   Include capital lock-up and time-to-resolution in ranking.
-   Require a minimum expected edge above estimated costs and
    uncertainty.
-   Fail closed when fee schedules, market rules, or required price
    levels are unknown.
-   Provide an operator kill switch and adapter disable switch.
-   Use idempotency keys for order-like operations.
-   Record every state transition and rejection reason.

### 9.2 Future live-execution controls

Before enabling live execution, implement and independently test: -
explicit human approval or documented strategy authorization; -
pre-trade buying-power and exposure checks; - order limits, price
collars, and maximum slippage; - idempotent order submission and
reconciliation; - cancel/replace state machine; - partial-fill and
duplicate-fill handling; - venue outage and disconnect behavior; -
emergency cancellation/kill switch; - credential rotation and
least-privilege keys; - daily reconciliation against venue balances,
orders, fills, and settlements; - regulatory, geographic, tax, and
venue-term review.

Never infer that a submitted order was filled. Confirm status from venue
data and reconcile after reconnects.

------------------------------------------------------------------------

## 10. Configuration

Use validated settings loaded from environment variables and/or a
non-secret YAML file. Secrets must be injected separately.

Example `config.example.yaml`:

``` yaml
app:
  environment: development
  mode: paper
  timezone: UTC
  log_level: INFO

venues:
  kalshi:
    enabled: false
    rest_base_url: ""
    websocket_url: ""
    request_timeout_seconds: 10
    max_retries: 4
  polymarket:
    enabled: false
    rest_base_url: ""
    websocket_url: ""
    request_timeout_seconds: 10
    max_retries: 4

market_data:
  max_book_age_ms: 3000
  snapshot_interval_ms: 1000
  minimum_depth_notional: "100"
  maximum_spread: "0.08"
  require_complete_book: true

signals:
  minimum_conservative_net_ev: "0.02"
  minimum_model_sample_count: 100
  maximum_probability_uncertainty: "0.10"
  candidate_ttl_seconds: 60
  allow_unreviewed_cross_market_links: false

risk:
  max_market_exposure: "25"
  max_event_exposure: "50"
  max_category_exposure: "100"
  max_total_exposure: "250"
  max_daily_loss: "25"
  require_human_approval: true

storage:
  database_url_env: DATABASE_URL
  raw_payload_retention_days: 365

execution:
  live_enabled: false
```

These are example development values, not recommended trading limits.
Choose thresholds based on account size, venue rules, liquidity, and
empirical testing. Validate all settings at startup and refuse to start
if `mode=live` while the live-execution feature is not explicitly
enabled and reviewed.

------------------------------------------------------------------------

## 11. Environment variables and secrets

Example `.env.example` (placeholders only):

``` dotenv
APP_ENV=development
APP_MODE=paper
DATABASE_URL=postgresql://user:password@localhost:5432/quantdb
REDIS_URL=redis://localhost:6379/0

KALSHI_API_KEY_ID=
KALSHI_PRIVATE_KEY_FILE=

POLYMARKET_API_KEY=
POLYMARKET_API_SECRET=
POLYMARKET_API_PASSPHRASE=
POLYMARKET_SIGNING_KEY=

LOG_LEVEL=INFO
```

Actual authentication requirements differ by venue and API operation.
Use official documentation to confirm credential formats and signing
requirements. Do not commit `.env`, private keys, wallet secrets, or
production credentials. Restrict key permissions and never expose
secrets to the browser or dashboard.

------------------------------------------------------------------------

## 12. Suggested repository layout

``` text
prediction-quant/
├── SPECS.md
├── README.md
├── Cargo.toml                 # Rust workspace manifest
├── Cargo.lock
├── rust-toolchain.toml
├── Dockerfile
├── compose.yaml
├── .env.example
├── config/
│   └── config.example.yaml
├── migrations/                # SQLx migrations
├── crates/
│   ├── domain/                # Typed domain models and invariants
│   ├── adapters/              # Kalshi and Polymarket API clients
│   ├── ingestion/             # Polling, WebSocket loops, normalization
│   ├── storage/               # SQLx repositories
│   ├── features/              # Point-in-time feature builders
│   ├── models/                # Probability models and calibration
│   ├── signals/               # Value, arbitrage, consistency, ranking
│   ├── risk/                  # Risk gates and exposure limits
│   ├── paper/                 # Paper orders, fills, settlement
│   ├── api/                   # Axum application and routes
│   └── observability/         # Tracing and metrics
├── frontend/                  # Next.js + TypeScript
│   ├── package.json
│   ├── app/
│   ├── components/
│   ├── lib/api-client.ts
│   └── tests/
├── tests/
│   ├── integration/
│   ├── contract-fixtures/
│   └── backtest-fixtures/
├── research/                  # Versioned datasets and experiment notes
├── data/
│   └── .gitkeep
└── scripts/
```

## 13. Testing requirements

### Unit tests (Rust)

-   Probability and payout math at boundary values 0 and 1.
-   Decimal rounding and tick-size handling.
-   Fee calculators per versioned schedule.
-   Order-book VWAP and insufficient-depth handling.
-   Candidate expiry and stale-data rejection.
-   Complement and implication constraint checks.
-   Ranking score determinism.
-   Risk limits and exposure aggregation.

### Property tests

-   Expected value is monotonic in probability for positive-payout YES
    contracts.
-   Increasing buy price cannot improve EV, all else equal.
-   Increasing costs cannot improve net EV.
-   Complementary probabilities remain within \[0,1\] and sum to 1 under
    the defined policy.
-   A rejected risk gate cannot become approved due only to a higher
    ranking score.

### Integration / contract tests (Rust API and Next.js client)

-   Recorded API fixtures normalize into the canonical schema.
-   Pagination and reconnect behavior.
-   WebSocket sequence-gap recovery.
-   API/database migrations.
-   End-to-end signal generation from a fixed snapshot.
-   Paper order lifecycle and settlement.
-   Adapter outage and stale-data behavior.

### Backtest regression tests

-   Fixed dataset and configuration produce reproducible results within
    defined numeric tolerances.
-   No feature timestamp exceeds the decision timestamp.
-   No signal uses a market rule version published after its decision
    time.
-   Cancelled/resolved markets follow explicit payout handling.

------------------------------------------------------------------------

## 14. Observability and operational metrics

Monitor at minimum: - adapter request latency, error rate, rate-limit
responses, reconnects; - latest market-data age by venue and market; -
snapshot completeness and sequence-gap counts; - markets
discovered/updated per minute; - feature and model inference latency; -
candidate count and rejection reasons; - fee/slippage estimates and
missing-fee events; - model calibration metrics and sample sizes; -
paper orders, fill assumptions, exposure, realized/unrealized P&L; -
database latency, queue lag, process memory, CPU, and restart counts.

Use structured logs with `request_id`, `venue`, `market_id`,
`signal_id`, `model_version`, and `decision_at`. Do not log secrets or
private account data.

------------------------------------------------------------------------

## 15. Delivery plan

### Milestone 0 --- Research and API verification

-   Confirm challenge requirements and intended deployment.
-   Read official Kalshi and Polymarket API documentation.
-   Verify access, account eligibility, geographic availability, fees,
    and market data permissions.
-   Select one narrow market category for the first model.
-   Document settlement matching rules.

**Exit:** documented API contracts and a reproducible data sample from
each enabled venue.

### Milestone 1 --- Canonical data and ingestion

-   Define Serde models and migrations.
-   Implement adapters and fixture tests.
-   Ingest markets, metadata, order books, and trades.
-   Persist raw and normalized records.

**Exit:** a query can reconstruct current and historical snapshots with
timestamps and provenance.

### Milestone 2 --- Baseline signals

-   Implement market-implied probability and basic value EV
    calculations.
-   Implement executable-side prices, fee interface, liquidity checks,
    and stale-data gates.
-   Expose `/signals` and a basic dashboard.

**Exit:** deterministic signals on fixture data, with a traceable
explanation and rejection reasons.

### Milestone 3 --- Paper trading and backtesting

-   Implement point-in-time replay.
-   Add realistic fill assumptions and fee/slippage calculations.
-   Report calibration, net P&L, drawdown, turnover, and baseline
    comparisons.

**Exit:** walk-forward evaluation on a holdout period with no detected
look-ahead leakage.

### Milestone 4 --- Cross-market and event-driven signals

-   Add reviewed market links and payoff-matrix validation.
-   Add external data adapters with provenance.
-   Add category-specific probability models.

**Exit:** cross-market alerts require approved semantic relationships;
event-driven models pass category-specific validation.

### Milestone 5 --- Operational hardening

-   Load tests, alerting, backups, access control, secrets management,
    failure drills.
-   Review venue terms, eligibility, and compliance requirements.
-   Conduct independent review of live-execution controls.

**Exit:** written operational sign-off. Live trading remains disabled
until this is complete.

------------------------------------------------------------------------

## 16. Definition of done

The MVP is complete when: - \[ \] Market metadata and books can be
collected from at least one venue; the second adapter is tested
independently. - \[ \] Canonical schemas and migrations are documented
and tested. - \[ \] Historical snapshots preserve point-in-time
availability. - \[ \] Value signals use executable prices and versioned
fee/slippage estimates. - \[ \] Signals include conservative EV,
rationale, provenance, freshness, and model/scoring versions. - \[ \]
Stale/incomplete data and unreviewed semantic matches fail closed. - \[
\] Dashboard/API supports ranking, filtering, and signal detail. - \[ \]
Paper trading and point-in-time backtesting are reproducible. - \[ \]
Model calibration and baseline comparisons are reported. - \[ \] Risk
limits, audit trail, and kill switch exist. - \[ \] Live execution is
disabled by default and excluded from MVP acceptance.

------------------------------------------------------------------------

## 17. Official documentation to verify during implementation

API endpoints, authentication methods, rate limits, fees, contract
formats, and product availability change. Before implementing adapters,
consult current official documentation rather than relying on copied
examples.

-   Kalshi API documentation: https://docs.kalshi.com/
-   Polymarket developer documentation: https://docs.polymarket.com/
-   Polymarket CLOB documentation:
    https://docs.polymarket.com/developers/CLOB/introduction
-   PostgreSQL documentation: https://www.postgresql.org/docs/
-   Axum documentation: https://docs.rs/axum/
-   Tokio documentation: https://tokio.rs/
-   SQLx documentation: https://docs.rs/sqlx/
-   Serde documentation: https://serde.rs/
-   Next.js documentation: https://nextjs.org/docs/
-   TypeScript documentation: https://www.typescriptlang.org/docs/

------------------------------------------------------------------------

## 18. Key engineering decisions to keep explicit

1.  **Market matching:** automated title similarity is a discovery aid,
    not proof of equivalent settlement.
2.  **Probability source:** every model estimate must be calibrated,
    versioned, and compared with a market baseline.
3.  **Cost model:** costs must be venue-specific, time-versioned, and
    computed from current official rules.
4.  **Execution realism:** a displayed price is not a guaranteed fill.
5.  **Data availability:** `published_at`, `observed_at`, and
    `ingested_at` are distinct.
6.  **Settlement:** cancellation, voiding, partial outcomes, and unusual
    payout rules require explicit handling.
7.  **Risk:** hard constraints are applied before ranking.
8.  **Live mode:** cannot be activated without a separate review and
    tested operational controls.
9.  **Compliance:** availability and legal eligibility depend on the
    user's location, account, and applicable rules; verify before live
    use.
10. **Iteration:** begin with one venue and one category if needed;
    preserve abstractions so the second venue and more models can be
    added without rewriting the signal engine.

------------------------------------------------------------------------

## 19. Rust and Next.js implementation standards

### 19.1 Rust workspace and crate boundaries

Use a Cargo workspace. Keep domain logic independent from Axum, SQLx,
and venue clients so that calculations can be unit-tested without a
database or network.

Recommended core dependencies (pin compatible versions in `Cargo.toml`;
verify current versions when implementation begins): - `tokio`:
asynchronous runtime. - `axum`: HTTP API. - `serde`, `serde_json`:
serialization. - `reqwest`: REST clients. - `tokio-tungstenite`:
WebSocket clients where needed. - `sqlx`: PostgreSQL and migrations. -
`rust_decimal`: exact decimal arithmetic for money and contract
prices. - `chrono`: timestamps. - `uuid`: internal IDs. - `async-trait`:
async adapter trait if required by the selected Rust version and trait
design. - `thiserror` / `anyhow`: typed library errors and application
error context. - `tracing`, `tracing-subscriber`: structured
observability. - `tower`, `tower-http`: middleware, timeouts, request
IDs, and CORS. - `utoipa` (optional): OpenAPI generation. - `proptest`:
property-based tests. - `wiremock` or equivalent: HTTP adapter tests.

Do not use `f64` for monetary values, fees, price ticks, or settlement
P&L. For statistical model internals, floating point can be appropriate,
but conversions to/from `Decimal` must be explicit, bounded, and tested.

### 19.2 Next.js frontend requirements

The frontend is a separate Next.js + TypeScript application. It must not
contain venue secrets, signing keys, or business logic that
authoritatively approves a trade.

Required pages: - `/`: system health, active venues, latest data
timestamps, and summary metrics. - `/markets`: searchable/filterable
market list. - `/markets/[id]`: market metadata, settlement rules, book,
and recent history. - `/signals`: ranked candidates with filters for
venue, category, signal type, net EV, liquidity, freshness, and
status. - `/signals/[id]`: detailed probability estimate, cost
breakdown, rationale, source evidence, risk gates, and model versions. -
`/paper`: paper orders, fills, positions, and P&L. - `/research`: model
versions and backtest reports (can be read-only in MVP). - `/settings`:
non-secret strategy thresholds and adapter status, with role-based
access if exposed beyond localhost.

Frontend behavior: - Display timestamps with an explicit timezone; store
and transmit timestamps in UTC. - Show stale/incomplete data prominently
and never visually present a blocked candidate as actionable. - Separate
gross EV from net EV and clearly identify estimates. - Show why
candidates were rejected as well as why they were ranked. - Include
loading, empty, partial-data, and API-error states. - Validate filters
client-side for usability, but enforce all safety and risk checks in the
Rust backend. - Use a generated OpenAPI TypeScript client where
practical; otherwise keep typed API interfaces in
`frontend/lib/api-client.ts`. - Use server components for read-mostly
page shells and client components for interactive filters/order-book
refreshes. - Protect private pages with authentication before any
non-local deployment. - Do not persist API secrets in browser storage or
expose them through `NEXT_PUBLIC_*` variables.

### 19.3 Frontend API contract

The Next.js frontend communicates only with the Rust API. In
development, configure a same-origin proxy or a single `API_BASE_URL` on
the server side. Avoid browser-side cross-origin requests where a
same-origin reverse proxy is practical.

Suggested environment: - Rust API: `API_BIND=0.0.0.0:8080` - Next.js
server-side API URL: `API_BASE_URL=http://api:8080` in Docker Compose;
`http://localhost:8080` for local non-container development. - Public
browser configuration may expose only non-secret UI settings. Never
expose database URLs, venue credentials, signing keys, or privileged
internal endpoints.

All API responses must have stable JSON shapes. Generate and check in an
OpenAPI document during CI, and fail CI if the API contract changes
without an intentional versioned update.

### 19.4 Docker Compose services

The development compose file should define: - `api`: Rust service built
from the Cargo workspace. - `frontend`: Next.js development/production
service. - `postgres`: persistent local PostgreSQL volume with health
check. - `redis`: optional cache/queue, disabled unless needed.

Use health checks and startup dependency conditions. Do not bake `.env`
or credentials into images. The Rust API should run database migrations
through a controlled startup or release step, not race multiple replicas
running the same migration simultaneously.

### 19.5 Additional acceptance criteria for the selected stack

-   `cargo fmt --check`,
    `cargo clippy --workspace --all-targets -- -D warnings`, and
    `cargo test --workspace` pass in CI.
-   The frontend passes lint, type-check, and production build.
-   OpenAPI schema and frontend API types remain synchronized.
-   The application runs locally through Docker Compose using documented
    setup steps.
-   No production secrets are present in repository history, frontend
    bundles, test fixtures, or logs.

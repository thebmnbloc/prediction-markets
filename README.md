# Polymarket Quant System — Voluntas Engineering Challenge

A Rust + Next.js decision-support platform for discovering, scoring, and paper-trading prediction-market opportunities on Polymarket.

## Purpose

Build a system that continuously discovers, evaluates, ranks, and explains prediction-market trading opportunities. The first release is a **decision-support system**: it produces ranked candidates and evidence; a human approves any live order. Automated execution is explicitly out of scope for the MVP.

## Scope (MVP)

- Ingest markets, order books, trades, metadata, fees, settlement rules, and status from Polymarket
- Normalize venue data into a canonical schema while preserving raw payloads
- Preserve point-in-time snapshots and provenance
- Estimate fair probabilities using transparent baseline models
- Detect value, cross-market, logical-consistency, and event-driven signals
- Calculate net EV, uncertainty-adjusted edge, costs, liquidity limits, and risk flags
- Rank opportunities with explainable rationale
- Provide dashboard/API for filtering, inspection, paper trading, and review
- Fail safely on stale, incomplete, or contradictory data

## Non-goals (MVP)

- Unsupervised autonomous trading
- Guaranteed-profit claims
- Treating every price difference as arbitrage
- High-frequency market making
- LLM-only probability or order authorization

## Architecture

Polymarket Adapter → Ingestion → Normalizer → Raw + Canonical Store
External Data/News → Source Adapters → Feature Builder → Probability Models
Opportunity Engine → Value / Cross-Market / Consistency / Event Signals → Risk Gate → Ranked Candidates
API / Dashboard / Alerts → Human Approval (later) → Order Manager (disabled MVP)

## Stack

- Rust (Tokio, Axum, SQLx, rust_decimal, tracing)
- PostgreSQL 16+ (schema + migrations)
- Next.js + TypeScript (dashboard)
- OpenAPI contract (`/openapi.json`)

## Safety

- Dry-run is the default; live execution is disabled in MVP
- No private keys in source control or logs
- Risk gates override ranking
- Stale/incomplete data fails closed
- Secrets are redacted in debug output

## Quick start

Backend (Rust):

```bash
cargo run -p pm-server
# API: http://127.0.0.1:3001
```

Frontend (Next.js):

```bash
cd frontend
npm install
npm run dev
# UI: http://localhost:3000
```

## Tests

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd frontend && npm run build
```

## Docker (optional)

```bash
docker compose up -d --build
```

## Configuration

Copy `.env.example` to `.env`. Market data is public and requires no keys.

For trading (dry-run by default):

- `POLYMARKET_PRIVATE_KEY` (Polygon wallet private key)
- `POLYMARKET_ENV=production`
- `DRY_RUN=true|false`
- `DATABASE_URL=postgres://quant:quant@localhost:5432/quantdb`

## Key endpoints

- `GET /health`, `GET /ready`
- `GET /markets/polymarket`
- `GET /orderbook/polymarket/{condition_id}`
- `GET /signals`, `POST /signals/refresh`, `GET /signals/{id}`
- `POST /paper/orders`, `GET /paper/orders`, `GET /paper/positions`
- `POST /consistency/check`
- `POST /backtest/run`
- `GET /news?q=...`
- `GET /cross-market-links`
- `GET /audit-events`
- `GET /openapi.json`
- `WS /ws`

## Frontend pages

- `/` — dashboard (markets, order book, balance, positions, news)
- `/signals` — ranked signals with refresh
- `/paper` — paper orders and positions
- `/research` — model/backtest placeholders
- `/settings` — readiness and configuration

## Roadmap

1. Canonical data + ingestion (done)
2. Baseline signals (done)
3. Paper trading + backtesting (in progress)
4. Cross-market + event-driven signals (in progress)
5. Operational hardening

## Compliance

Verify account eligibility, geographic restrictions, venue terms, and regulatory obligations before any live use.
```<|end|><|start|>write to=assistant<|channel|>analysis<|message|>Created file successfully: C:/Users/BIOR/projects/prediction-markets/README.md
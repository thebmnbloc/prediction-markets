# Polymarket Quant System (Voluntas Engineering Challenge)

A Rust + Next.js decision-support platform for discovering, scoring, and paper-trading prediction-market opportunities on Polymarket.

## Quick start

### Backend (Rust)
```bash
cargo run -p pm-server
# API: http://127.0.0.1:3001
```

### Frontend (Next.js)
```bash
cd frontend
npm install
npm run dev
# UI: http://localhost:3000
```

### Tests
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd frontend && npm run build
```

### Docker (optional)
```bash
docker compose up -d --build
```

## Configuration
Copy `.env.example` to `.env`. Market data is public and requires no keys.

For trading (dry-run by default):
- `POLYMARKET_PRIVATE_KEY` (Polygon wallet private key)
- `POLYMARKET_ENV=production`
- `DRY_RUN=true|false`

## Key endpoints
- `GET /health`, `GET /ready`
- `GET /markets/polymarket`
- `GET /orderbook/polymarket/{condition_id}`
- `GET /signals`, `POST /signals/refresh`, `GET /signals/{id}`
- `POST /paper/orders`, `GET /paper/orders`
- `POST /consistency/check`
- `GET /news?q=...`
- `GET /openapi.json`
- `WS /ws`

## Safety
- Dry-run is the default; live execution is not enabled in MVP.
- No secrets are logged or exposed to the browser.
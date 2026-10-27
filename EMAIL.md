# Email to Voluntas

Subject: Polymarket Quant System – Engineering Challenge Submission (Voluntas)

Hi Voluntas team,

Thank you for the opportunity. I’m submitting my engineering challenge solution, a Polymarket-focused quant decision-support system built in Rust (backend) and Next.js (frontend). This is my first deep dive into the Polymarket APIs, and I focused on building a solid, testable foundation rather than over-claiming capabilities.

Repository: https://github.com/thebmnbloc/prediction-markets

What’s implemented
- Polymarket adapter using Gamma (market discovery) and CLOB (order books, prices, spreads)
- Canonical domain model with Decimal-only financial math
- Risk gates (stale/incomplete data, liquidity, exposure) that fail closed
- Signal engine with executable-side pricing, EV, conservative EV, and ranking
- Paper trading + audit events + kill switch (dry-run by default)
- Cross-market consistency checks and a basic backtest endpoint
- Next.js dashboard with market search, live order books, signals, paper orders, news, and category filters
- OpenAPI contract (/openapi.json) and Docker Compose setup
- PostgreSQL persistence (schema + migrations for trades/signals/paper/audit)

What’s intentionally not in MVP
- Live order execution (disabled by design; requires credential + compliance review)
- Full point-in-time backtesting and calibration reporting
- Cross-market payoff matrices and settlement compatibility verification
- Category-specific probability models beyond baseline

Time constraint note
The deadline is today and this is my first Polymarket integration, so I prioritized correctness, safety, and reproducibility over breadth. The codebase is structured so the remaining milestones can be added incrementally without rewriting the signal engine.

How to run
cargo run -p pm-server
cd frontend && npm install && npm run dev

I’d welcome feedback on architecture, risk controls, and model validation. Thank you for your time.

Best regards,
Bior
```<|end|><|start|>write to=assistant<|channel|>analysis<|message|>Wrote file successfully: C:/Users/BIOR/projects/prediction-markets/EMAIL.md
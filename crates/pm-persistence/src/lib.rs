use pm_core::errors::PmError;
use pm_core::types::Fill;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

pub struct TradeStore {
    pub pool: PgPool,
}

impl TradeStore {
    pub async fn new(database_url: &str) -> Result<Self, PmError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| PmError::database(e.to_string()))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub async fn migrate(&self) -> Result<(), PmError> {
        let stmts = [
            "CREATE TABLE IF NOT EXISTS venues (id UUID PRIMARY KEY, code TEXT UNIQUE NOT NULL, display_name TEXT NOT NULL, enabled BOOLEAN NOT NULL DEFAULT TRUE, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS markets (id UUID PRIMARY KEY, venue_market_id TEXT NOT NULL, title TEXT NOT NULL, status TEXT NOT NULL, last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS trades (id SERIAL PRIMARY KEY, order_id TEXT NOT NULL, exchange TEXT NOT NULL, market_id TEXT NOT NULL, side TEXT NOT NULL, price TEXT NOT NULL, quantity TEXT NOT NULL, timestamp TIMESTAMPTZ NOT NULL)",
            "CREATE TABLE IF NOT EXISTS signals (id UUID PRIMARY KEY, market_id UUID, signal_type TEXT NOT NULL, side TEXT NOT NULL, net_ev TEXT NOT NULL, score DOUBLE PRECISION NOT NULL, status TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS paper_orders (id UUID PRIMARY KEY, market_id UUID, side TEXT NOT NULL, limit_price TEXT NOT NULL, requested_size TEXT NOT NULL, status TEXT NOT NULL, submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS audit_events (id UUID PRIMARY KEY, actor_type TEXT NOT NULL, action TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, details JSONB NOT NULL DEFAULT '{}'::jsonb, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
        ];
        for stmt in stmts {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| PmError::database(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn record_fill(&self, fill: &Fill) -> Result<(), PmError> {
        sqlx::query(
            "INSERT INTO trades (order_id, exchange, market_id, side, price, quantity, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&fill.order_id)
        .bind(fill.exchange.to_string())
        .bind(fill.market_id.to_string())
        .bind(fill.side.to_string())
        .bind(fill.price.to_string())
        .bind(fill.quantity.to_string())
        .bind(fill.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| PmError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn count_trades(&self) -> Result<i64, PmError> {
        let row = sqlx::query("SELECT COUNT(*) as c FROM trades")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PmError::database(e.to_string()))?;
        Ok(row.get("c"))
    }
}
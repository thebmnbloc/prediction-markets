use pm_core::AppConfig;
use pm_server::{app_with_state, AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = AppConfig::load().unwrap_or_default();
    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port)
        .parse()
        .unwrap();
    let state = AppState::new(cfg);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("pm-server listening on {}", addr);
    axum::serve(listener, app_with_state(state)).await.unwrap();
}

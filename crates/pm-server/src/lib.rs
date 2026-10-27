use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use pm_core::traits::ExchangeAdapter;
use pm_core::types::{Exchange, MarketFilter, NewOrder};
use pm_core::RiskCheck;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::OpenApi;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub mod state;
pub use state::AppState;

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

pub fn app() -> Router {
    app_with_state(AppState::new(pm_core::AppConfig::default()))
}

pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/openapi.json", get(openapi))
        .route("/markets/:exchange", get(list_markets))
        .route("/markets/:exchange/:id", get(get_market))
        .route("/orderbook/:exchange/:id", get(get_orderbook))
        .route("/positions", get(get_positions))
        .route("/balance/:exchange", get(get_balance))
        .route("/orders", post(place_order))
        .route("/orders/:id", delete(cancel_order))
        .route("/ws", get(ws_handler))
        .route("/news", get(news))
        .route("/signals", get(list_signals))
        .route("/signals/refresh", post(refresh_signals))
        .route(
            "/paper/orders",
            get(list_paper_orders).post(create_paper_order),
        )
        .route("/signals/:id", get(get_signal))
        .route("/paper/positions", get(paper_positions))
        .route("/audit-events", get(list_audit_events))
        .route(
            "/cross-market-links",
            get(list_cross_links).post(create_cross_link),
        )
        .route("/consistency/check", post(check_consistency))
        .with_state(state)
        .layer(cors())
}

#[utoipa::path(get, path = "/health", responses((status = 200, description = "health")))]
async fn health() -> &'static str {
    "ok"
}

async fn ready(State(state): State<AppState>) -> StatusCode {
    if state.store.is_some() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

fn parse_exchange(s: &str) -> Result<Exchange, StatusCode> {
    Exchange::from_str(s).map_err(|_| StatusCode::BAD_REQUEST)
}

fn adapter_for(
    state: &AppState,
    ex: Exchange,
) -> Result<std::sync::Arc<dyn pm_core::traits::ExchangeAdapter>, StatusCode> {
    match ex {
        Exchange::Polymarket => {
            Ok(state.polymarket.clone() as Arc<dyn pm_core::traits::ExchangeAdapter>)
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn list_markets(
    State(state): State<AppState>,
    Path(exchange): Path<String>,
) -> Result<Json<Vec<pm_core::types::Market>>, StatusCode> {
    let ex = parse_exchange(&exchange)?;
    let adapter = adapter_for(&state, ex)?;
    let filter = MarketFilter::default();
    let markets = adapter
        .get_markets(&filter)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(markets))
}

async fn get_market(
    State(state): State<AppState>,
    Path((exchange, id)): Path<(String, String)>,
) -> Result<Json<pm_core::types::Market>, StatusCode> {
    let ex = parse_exchange(&exchange)?;
    let adapter = adapter_for(&state, ex)?;
    let market = adapter
        .get_market(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(market))
}

async fn get_orderbook(
    State(state): State<AppState>,
    Path((exchange, id)): Path<(String, String)>,
) -> Result<Json<pm_core::types::OrderBook>, StatusCode> {
    let ex = parse_exchange(&exchange)?;
    let adapter = adapter_for(&state, ex)?;
    let book = adapter
        .get_orderbook(&id)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(book))
}

async fn get_positions(
    State(state): State<AppState>,
) -> Result<Json<Vec<pm_core::types::Position>>, StatusCode> {
    let positions = state.polymarket.get_positions().await.unwrap_or_default();
    Ok(Json(positions))
}

async fn get_balance(
    State(state): State<AppState>,
    Path(exchange): Path<String>,
) -> Result<Json<pm_core::types::Balance>, StatusCode> {
    let ex = parse_exchange(&exchange)?;
    let adapter = adapter_for(&state, ex)?;
    let bal = adapter
        .get_balance()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(bal))
}

async fn place_order(
    State(state): State<AppState>,
    Json(order): Json<NewOrder>,
) -> Result<Json<pm_core::types::OrderConfirmation>, StatusCode> {
    let ex = order.market_id.exchange;
    let adapter = adapter_for(&state, ex)?;
    // risk check
    {
        let risk = state.risk.clone();
        risk.check_order(&order)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }
    if state.config.general.dry_run {
        // simulate
        return Ok(Json(pm_core::types::OrderConfirmation {
            order_id: "dry-run".into(),
            exchange: ex,
            market_id: order.market_id.clone(),
            status: pm_core::types::OrderStatus::Open,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: None,
            timestamp: chrono::Utc::now(),
        }));
    }
    let mut exec = state.execution.lock().await;
    exec.route_order(&order)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let conf = adapter
        .place_order(&order)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(conf))
}

async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let _ = state.polymarket.cancel_order(&id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct NewsQuery {
    q: Option<String>,
}

async fn news(Query(params): Query<NewsQuery>) -> Result<Json<serde_json::Value>, StatusCode> {
    let q = params
        .q
        .unwrap_or_else(|| "polymarket prediction market".to_string());
    let url = format!(
        "https://api.gdeltproject.org/api/v2/doc/doc?query={}&mode=artlist&maxrecords=15&format=json",
        q.replace(' ', "%20")
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let v: serde_json::Value = resp.json().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(v))
}

#[utoipa::path(get, path = "/signals", responses((status = 200, description = "signals", body = [crate::state::SignalRecord])))]
async fn list_signals(State(state): State<AppState>) -> Json<Vec<crate::state::SignalRecord>> {
    let sigs = state.signals.lock().await;
    Json(sigs.clone())
}

async fn refresh_signals(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::state::SignalRecord>>, StatusCode> {
    let markets = state
        .polymarket
        .get_markets(&pm_core::types::MarketFilter::default())
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut out = Vec::new();
    for m in markets.into_iter().take(25) {
        if let Ok(book) = state.polymarket.get_orderbook(&m.id.id).await {
            if let Some(ask) = book.asks.first() {
                let price: rust_decimal::Decimal = ask.price;
                let p = rust_decimal::Decimal::new(55, 2); // 0.55 model prob
                let fee = rust_decimal::Decimal::new(1, 2); // 0.01
                let ev = p - price - fee;
                if ev > rust_decimal::Decimal::ZERO {
                    out.push(crate::state::SignalRecord {
                        id: format!("sig-{}", m.id.id),
                        market_id: m.id.id.clone(),
                        title: m.title.clone(),
                        side: "buy".to_string(),
                        executable_price: price.to_string(),
                        model_probability: p.to_string(),
                        net_ev: ev.to_string(),
                        status: "candidate".to_string(),
                    });
                }
            }
        }
    }
    let mut sigs = state.signals.lock().await;
    *sigs = out.clone();
    drop(sigs);
    let mut audits = state.audit_events.lock().await;
    let n = audits.len();
    audits.push(crate::state::AuditEvent {
        id: format!("audit-{}", n),
        actor_type: "system".to_string(),
        action: "signals_refreshed".to_string(),
        entity_type: "signal".to_string(),
        entity_id: "batch".to_string(),
        details: format!("{} candidates", out.len()),
        created_at: chrono::Utc::now().to_rfc3339(),
    });
    Ok(Json(out))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct PaperOrderReq {
    market_id: String,
    side: String,
    price: String,
    size: String,
}

#[utoipa::path(post, path = "/paper/orders", request_body = PaperOrderReq, responses((status = 200, description = "paper order", body = crate::state::PaperOrder)))]
async fn create_paper_order(
    State(state): State<AppState>,
    Json(req): Json<PaperOrderReq>,
) -> Json<crate::state::PaperOrder> {
    let order = crate::state::PaperOrder {
        id: format!("paper-{}", state.paper_orders.lock().await.len()),
        market_id: req.market_id,
        side: req.side,
        price: req.price,
        size: req.size,
        status: "open".to_string(),
    };
    state.paper_orders.lock().await.push(order.clone());
    let mut audits = state.audit_events.lock().await;
    let n = audits.len();
    audits.push(crate::state::AuditEvent {
        id: format!("audit-{}", n),
        actor_type: "user".to_string(),
        action: "paper_order_created".to_string(),
        entity_type: "paper_order".to_string(),
        entity_id: order.id.clone(),
        details: format!("{} {} @ {}", order.side, order.size, order.price),
        created_at: chrono::Utc::now().to_rfc3339(),
    });
    Json(order)
}

async fn list_paper_orders(State(state): State<AppState>) -> Json<Vec<crate::state::PaperOrder>> {
    let orders = state.paper_orders.lock().await;
    Json(orders.clone())
}

async fn get_signal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::state::SignalRecord>, StatusCode> {
    let sigs = state.signals.lock().await;
    sigs.iter()
        .find(|s| s.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct CrossLinkReq {
    market_a_id: String,
    market_b_id: String,
    relation_type: String,
    settlement_compatible: bool,
}

async fn list_cross_links(
    State(state): State<AppState>,
) -> Json<Vec<crate::state::CrossMarketLink>> {
    let links = state.cross_links.lock().await;
    Json(links.clone())
}

async fn create_cross_link(
    State(state): State<AppState>,
    Json(req): Json<CrossLinkReq>,
) -> Json<crate::state::CrossMarketLink> {
    let link = crate::state::CrossMarketLink {
        id: format!("link-{}", state.cross_links.lock().await.len()),
        market_a_id: req.market_a_id,
        market_b_id: req.market_b_id,
        relation_type: req.relation_type,
        settlement_compatible: req.settlement_compatible,
        review_status: "unreviewed".to_string(),
    };
    state.cross_links.lock().await.push(link.clone());
    Json(link)
}

#[derive(serde::Deserialize)]
struct ConsistencyReq {
    relation_type: String,
    p_a: String,
    p_b: String,
    tolerance: Option<String>,
}

async fn check_consistency(Json(req): Json<ConsistencyReq>) -> Json<serde_json::Value> {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    let pa = Decimal::from_str(&req.p_a).unwrap_or(Decimal::ZERO);
    let pb = Decimal::from_str(&req.p_b).unwrap_or(Decimal::ZERO);
    let tol = req
        .tolerance
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_else(|| Decimal::new(1, 2));
    let (consistent, violation) = match req.relation_type.as_str() {
        "equivalent" => {
            let diff = (pa - pb).abs();
            if diff <= tol {
                (true, None)
            } else {
                (false, Some(format!("|p_a - p_b| = {} > tol {}", diff, tol)))
            }
        }
        "implies" => {
            if pa <= pb + tol {
                (true, None)
            } else {
                (false, Some(format!("p_a = {} > p_b = {} + tol", pa, pb)))
            }
        }
        "mutually_exclusive" => {
            if pa + pb <= Decimal::ONE + tol {
                (true, None)
            } else {
                (false, Some(format!("p_a + p_b = {} > 1 + tol", pa + pb)))
            }
        }
        "complement" => {
            let sum = pa + pb;
            if (sum - Decimal::ONE).abs() <= tol {
                (true, None)
            } else {
                (false, Some(format!("p_a + p_b = {} != 1", sum)))
            }
        }
        _ => (true, Some("unknown relation type; no constraint applied".to_string())),
    };
    Json(serde_json::json!({
        "consistent": consistent,
        "violation": violation,
        "relation_type": req.relation_type,
        "p_a": pa.to_string(),
        "p_b": pb.to_string(),
        "tolerance": tol.to_string(),
    }))
}

async fn paper_positions(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    let orders = state.paper_orders.lock().await;
    // group by market
    let mut markets: std::collections::BTreeMap<String, (Decimal, Decimal, Decimal)> = std::collections::BTreeMap::new();
    // (net_qty, cost, count)
    for o in orders.iter() {
        let qty = Decimal::from_str(&o.size).unwrap_or(Decimal::ZERO);
        let price = Decimal::from_str(&o.price).unwrap_or(Decimal::ZERO);
        let signed = if o.side == "buy" { qty } else { -qty };
        let entry = markets.entry(o.market_id.clone()).or_insert((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
        entry.0 += signed;
        entry.1 += signed * price;
        entry.2 += qty;
    }
    let mut out = Vec::new();
    for (market_id, (net_qty, cost, _)) in markets {
        let avg = if net_qty != Decimal::ZERO { cost / net_qty } else { Decimal::ZERO };
        // fetch current mid
        let current = state
            .polymarket
            .get_orderbook(&market_id)
            .await
            .ok()
            .and_then(|b| {
                let bid = b.bids.first().map(|l| l.price);
                let ask = b.asks.first().map(|l| l.price);
                match (bid, ask) {
                    (Some(b), Some(a)) => Some((b + a) / Decimal::from(2)),
                    (Some(b), None) => Some(b),
                    (None, Some(a)) => Some(a),
                    _ => None,
                }
            })
            .unwrap_or(Decimal::ZERO);
        let unrealized = (current - avg) * net_qty;
        out.push(serde_json::json!({
            "market_id": market_id,
            "net_quantity": net_qty.to_string(),
            "avg_entry_price": avg.to_string(),
            "current_price": current.to_string(),
            "unrealized_pnl": unrealized.to_string(),
        }));
    }
    Json(out)
}

async fn list_audit_events(State(state): State<AppState>) -> Json<Vec<crate::state::AuditEvent>> {
    let events = state.audit_events.lock().await;
    Json(events.clone())
}

#[derive(OpenApi)]
#[openapi(
    paths(health, list_signals, create_paper_order),
    components(schemas(crate::state::SignalRecord, crate::state::PaperOrder, PaperOrderReq))
)]
struct ApiDoc;

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket) {
    use axum::extract::ws::Message;
    let _ = socket.send(Message::Text("connected".into())).await;
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                let _ = socket.send(Message::Text(format!("echo: {}", t))).await;
            }
            Message::Binary(b) => {
                let _ = socket.send(Message::Binary(b)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

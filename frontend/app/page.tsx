"use client";

import { useEffect, useMemo, useState } from "react";

type Market = { id: { exchange: string; id: string }; title: string; volume: string };
type Position = { market_id: { id: string }; side: string; quantity: string };
type Level = { price: string; quantity: string };
type OrderBook = { bids: Level[]; asks: Level[] };
type Balance = { available: string; reserved: string; total: string };

const API = "http://localhost:3001";

export default function Home() {
  const [health, setHealth] = useState("checking...");
  const [markets, setMarkets] = useState<Market[]>([]);
  const [news, setNews] = useState<any[]>([]);
  const [positions, setPositions] = useState<Position[]>([]);
  const [balance, setBalance] = useState<Balance | null>(null);
  const [selected, setSelected] = useState("");
  const [book, setBook] = useState<OrderBook | null>(null);
  const [query, setQuery] = useState("");
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [price, setPrice] = useState("0.50");
  const [qty, setQty] = useState("5");
  const [orderResult, setOrderResult] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    fetch(`${API}/health`).then((r) => r.text()).then(setHealth).catch(() => setHealth("unreachable"));
    fetch(`${API}/markets/polymarket`).then((r) => r.json()).then(setMarkets).catch((e) => setError(String(e)));
    fetch(`${API}/positions`).then((r) => r.json()).then(setPositions).catch(() => {});
    fetch(`${API}/balance/polymarket`).then((r) => r.json()).then(setBalance).catch(() => {});
    fetch(`${API}/news?q=polymarket`).then((r) => r.json()).then((d) => setNews(d.articles || [])).catch(() => {});
  }, []);

  useEffect(() => {
    if (!selected) return;
    fetch(`${API}/orderbook/polymarket/${selected}`)
      .then((r) => r.json())
      .then(setBook)
      .catch(() => setBook(null));
  }, [selected]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return [...markets]
      .sort((a, b) => Number(b.volume || 0) - Number(a.volume || 0))
      .filter((m) => (m.title || m.id.id).toLowerCase().includes(q))
      .slice(0, 60);
  }, [markets, query]);

  const selectedMarket = markets.find((m) => m.id.id === selected);

  async function placeOrder() {
    if (!selectedMarket) return;
    const order = {
      market_id: selectedMarket.id,
      side,
      order_type: "limit",
      price,
      quantity: qty,
    };
    const res = await fetch(`${API}/orders`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(order),
    });
    const data = await res.json();
    setOrderResult(JSON.stringify(data, null, 2));
  }

  const maxBidSize = Math.max(...(book?.bids.map((b) => Number(b.quantity)) || [1]));
  const maxAskSize = Math.max(...(book?.asks.map((a) => Number(a.quantity)) || [1]));

  return (
    <div className="app">
      <header className="header">
        <div className="brand">
          <div className="logo">PM</div>
          <div>
            <div className="title">Polymarket Quant</div>
            <div className="subtitle">Live markets, books, and execution</div>
          </div>
        </div>
        <div className="status-pill">
          API: <span className={health === "ok" ? "status-ok" : ""}>{health}</span>
        </div>
      </header>

      {error && <div className="card" style={{ borderColor: "#ef4444" }}>{error}</div>}

      <div className="grid">
        <div className="card span-4">
          <h2>Balance</h2>
          {balance ? (
            <div>
              <div className="kpi">${balance.available}</div>
              <div className="muted">Available</div>
              <div style={{ marginTop: 8 }}>Reserved: ${balance.reserved}</div>
              <div>Total: ${balance.total}</div>
            </div>
          ) : (
            <div className="muted">Loading...</div>
          )}
        </div>

        <div className="card span-8">
          <h2>Markets</h2>
          <input
            className="input"
            style={{ width: "100%", marginBottom: 10 }}
            placeholder="Search markets..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <div style={{ maxHeight: 320, overflow: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th>Market</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((m) => (
                  <tr key={m.id.id} style={{ background: selected === m.id.id ? "rgba(79,140,255,0.12)" : undefined }}>
                    <td>{m.title || m.id.id}</td>
                    <td>
                      <button onClick={() => setSelected(m.id.id)}>View</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <div className="card span-6">
          <h2>Order Book {selectedMarket ? `— ${selectedMarket.title}` : ""}</h2>
          {!book && <div className="muted">Select a market to view the book.</div>}
          {book && (
            <div className="book-grid">
              <div>
                <h3>Bids</h3>
                {book.bids.slice(0, 12).map((b, i) => (
                  <div key={i}>
                    <div className="level">
                      <span className="bid">{b.price}</span>
                      <span>{b.quantity}</span>
                    </div>
                    <div className="bar bid">
                      <div style={{ width: `${(Number(b.quantity) / maxBidSize) * 100}%` }} />
                    </div>
                  </div>
                ))}
              </div>
              <div>
                <h3>Asks</h3>
                {book.asks.slice(0, 12).map((a, i) => (
                  <div key={i}>
                    <div className="level">
                      <span className="ask">{a.price}</span>
                      <span>{a.quantity}</span>
                    </div>
                    <div className="bar ask">
                      <div style={{ width: `${(Number(a.quantity) / maxAskSize) * 100}%` }} />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="card span-6">
          <h2>Place Order</h2>
          <div className="row" style={{ marginBottom: 10 }}>
            <select value={side} onChange={(e) => setSide(e.target.value as "buy" | "sell")}>
              <option value="buy">Buy</option>
              <option value="sell">Sell</option>
            </select>
            <input className="input" value={price} onChange={(e) => setPrice(e.target.value)} placeholder="price" />
            <input className="input" value={qty} onChange={(e) => setQty(e.target.value)} placeholder="qty" />
            <button onClick={placeOrder}>Submit</button>
          </div>
          <div className="muted">Selected: {selected || "none"}</div>
          {orderResult && <pre>{orderResult}</pre>}
        </div>

        <div className="card span-6">
          <h2>Top News (GDELT)</h2>
          <div style={{ maxHeight: 260, overflow: "auto" }}>
            {news.slice(0, 12).map((a, i) => (
              <div key={i} style={{ marginBottom: 10 }}>
                <a href={a.url} target="_blank" rel="noreferrer">{a.title}</a>
                <div className="muted">{a.domain} · {a.seendate}</div>
              </div>
            ))}
            {news.length === 0 && <div className="muted">Loading news...</div>}
          </div>
        </div>

        <div className="card span-12">
          <h2>Positions</h2>
          {positions.length === 0 ? (
            <div className="muted">No positions.</div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Market</th>
                  <th>Side</th>
                  <th>Quantity</th>
                </tr>
              </thead>
              <tbody>
                {positions.map((p, i) => (
                  <tr key={i}>
                    <td>{p.market_id.id}</td>
                    <td>{p.side}</td>
                    <td>{p.quantity}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
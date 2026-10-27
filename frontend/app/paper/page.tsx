"use client";
import { useEffect, useState } from "react";
const API = "http://localhost:3001";
export default function PaperPage() {
  const [orders, setOrders] = useState<any[]>([]);
  const [positions, setPositions] = useState<any[]>([]);
  useEffect(() => {
    fetch(`${API}/paper/orders`).then((r) => r.json()).then(setOrders).catch(() => {});
    fetch(`${API}/paper/positions`).then((r) => r.json()).then(setPositions).catch(() => {});
  }, []);
  return (
    <main style={{ padding: 24 }}>
      <h1>Paper Trading</h1>
      <h2>Orders</h2>
      <ul>{orders.map((o) => <li key={o.id}>{o.id} {o.side} {o.size} @ {o.price} ({o.status})</li>)}</ul>
      <h2>Positions</h2>
      <ul>{positions.map((p, i) => <li key={i}>{p.market_id} qty={p.net_quantity} avg={p.avg_entry_price} unrealized={p.unrealized_pnl}</li>)}</ul>
    </main>
  );
}
"use client";
import { useEffect, useState } from "react";
const API = "http://localhost:3001";
export default function SignalsPage() {
  const [signals, setSignals] = useState<any[]>([]);
  useEffect(() => {
    fetch(`${API}/signals`).then((r) => r.json()).then(setSignals).catch(() => {});
  }, []);
  return (
    <main style={{ padding: 24 }}>
      <h1>Signals</h1>
      <button onClick={() => fetch(`${API}/signals/refresh`, { method: "POST" }).then((r) => r.json()).then(setSignals)}>Refresh</button>
      <table style={{ width: "100%", marginTop: 12, borderCollapse: "collapse" }}>
        <thead><tr><th>ID</th><th>Market</th><th>Side</th><th>Price</th><th>Prob</th><th>Net EV</th><th>Status</th></tr></thead>
        <tbody>
          {signals.map((s) => (
            <tr key={s.id}>
              <td>{s.id}</td>
              <td>{s.title}</td>
              <td>{s.side}</td>
              <td>{s.executable_price}</td>
              <td>{s.model_probability}</td>
              <td>{s.net_ev}</td>
              <td>{s.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {signals.length === 0 && <p>No signals. Click Refresh.</p>}
    </main>
  );
}
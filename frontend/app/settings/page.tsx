"use client";
import { useEffect, useState } from "react";
const API = "http://localhost:3001";
export default function SettingsPage() {
  const [ready, setReady] = useState("");
  useEffect(() => {
    fetch(`${API}/ready`).then((r) => r.text()).then(setReady).catch(() => setReady("unreachable"));
  }, []);
  return (
    <main style={{ padding: 24 }}>
      <h1>Settings</h1>
      <p>Readiness: {ready}</p>
      <p>Strategy thresholds and adapter status are configured via environment variables and the Rust backend.</p>
    </main>
  );
}
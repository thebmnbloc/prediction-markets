import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Prediction Markets Quant",
  description: "Monitoring dashboard",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <nav className="nav">
          <a href="/" className="brand">Polymarket Quant</a>
          <div className="nav-links">
            <a href="/">Dashboard</a>
            <a href="/signals">Signals</a>
            <a href="/paper">Paper</a>
            <a href="/research">Research</a>
            <a href="/settings">Settings</a>
          </div>
        </nav>
        {children}
      </body>
    </html>
  );
}
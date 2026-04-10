import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

export function App() {
  const [version, setVersion] = useState<string>("…");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion("unknown"));
  }, []);

  return (
    <div className="boot">
      <h1>cmux (rust rewrite)</h1>
      <p>Phase 1 skeleton — Tauri v2 + React + Vite.</p>
      <p>
        App version: <code>{version}</code>
      </p>
      <p>
        See <code>PLAN.md</code> at the repo root for the full phased rewrite
        plan.
      </p>
    </div>
  );
}

import { useEffect, useState } from "react";

import type { BrowserPanelProps } from "./browser-types";

export function BrowserPanel({ snapshot, onOpenUrl, onBack, onForward, onReload }: BrowserPanelProps) {
  const [url, setUrl] = useState(snapshot.current_url ?? "https://example.com");
  const [findText, setFindText] = useState("");

  useEffect(() => {
    setUrl(snapshot.current_url ?? "https://example.com");
  }, [snapshot.current_url]);

  const canOpen = Boolean(onOpenUrl) && url.trim().length > 0;

  return (
    <section className="browserPanel">
      <header className="browserPanelHeader">
        <div>
          <p className="eyebrow">Browser</p>
          <h3>{snapshot.title || "Detached browser window"}</h3>
        </div>
        <div className="browserPanelMeta">
          <span>{snapshot.current_url || "about:blank"}</span>
          <span>Zoom {snapshot.page_zoom.toFixed(2)}x</span>
        </div>
      </header>

      <div className="browserPanelToolbar">
        <button
          type="button"
          className="ghostButton"
          disabled={!onBack || snapshot.back_history.length === 0}
          onClick={() => {
            void onBack?.();
          }}
        >
          Back
        </button>
        <button
          type="button"
          className="ghostButton"
          disabled={!onForward || snapshot.forward_history.length === 0}
          onClick={() => {
            void onForward?.();
          }}
        >
          Forward
        </button>
        <button
          type="button"
          className="ghostButton"
          disabled={!onReload}
          onClick={() => {
            void onReload?.();
          }}
        >
          Reload
        </button>
        <input
          className="browserPanelInput"
          type="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://example.com"
        />
        <button
          type="button"
          className="primaryButton"
          disabled={!canOpen}
          onClick={() => {
            void onOpenUrl?.({ url, target_title: snapshot.title || "Browser" });
          }}
        >
          Open
        </button>
      </div>

      <div className="browserPanelFind">
        <input
          className="browserPanelInput browserPanelFindInput"
          type="search"
          value={findText}
          onChange={(event) => setFindText(event.target.value)}
          placeholder="Find in browser"
          aria-label="Search browser contents"
        />
        <button type="button" className="ghostButton" disabled>
          Prev
        </button>
        <button type="button" className="ghostButton" disabled>
          Next
        </button>
        <span>{findText.trim() ? "0 matches" : "Search ready"}</span>
      </div>

      <div className="browserPanelViewport">
        <div className="browserPanelGlass">
          <div className="browserPanelGlobe" />
          <div className="browserPanelCopy">
            <p>{snapshot.should_render_webview ? "Tauri WebviewWindow target" : "Webview hidden"}</p>
            <p>{snapshot.window_label}</p>
            <p>
              {snapshot.back_history.length} back / {snapshot.forward_history.length} forward
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

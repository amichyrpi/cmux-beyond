import { useEffect, useRef, useState } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal as XTerm } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import type { TerminalSurfaceProps } from "./terminal-types";

const terminalTheme = {
  background: "#09121b",
  foreground: "#edf4ff",
  cursor: "#7be7db",
  selectionBackground: "rgba(79, 209, 197, 0.28)",
  black: "#0b1016",
  blue: "#7da0ff",
  green: "#81e6a5",
  red: "#ff8d7a",
  yellow: "#ffd86e",
  magenta: "#d6a3ff",
  cyan: "#7be7db",
  white: "#edf4ff",
  brightBlack: "#5b6674",
  brightBlue: "#a7b7ff",
  brightGreen: "#b6f0c6",
  brightRed: "#ffb3a6",
  brightYellow: "#ffe8a8",
  brightMagenta: "#e7c7ff",
  brightCyan: "#b7f6ef",
  brightWhite: "#ffffff",
};

export function TerminalSurface({ snapshot, searchQuery, onInput }: TerminalSurfaceProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const onInputRef = useRef(onInput);
  const [searchText, setSearchText] = useState(searchQuery ?? "");
  const searchCount = searchText.trim()
    ? snapshot.visible_text
        .toLowerCase()
        .split(searchText.trim().toLowerCase())
        .length - 1
    : 0;

  useEffect(() => {
    onInputRef.current = onInput;
  }, [onInput]);

  useEffect(() => {
    setSearchText(searchQuery ?? "");
  }, [searchQuery]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return;
    }

    const terminal = new XTerm({
      convertEol: true,
      cursorBlink: true,
      fontFamily:
        '"SFMono-Regular", "Cascadia Mono", "JetBrains Mono", "SF Mono", Consolas, monospace',
      fontSize: 13,
      lineHeight: 1.2,
      theme: terminalTheme,
      rows: snapshot.rows,
      cols: snapshot.columns,
      scrollback: 4000,
      allowTransparency: true,
    });
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(host);
    terminal.focus();

    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    const dataListener = terminal.onData((data) => {
      void onInputRef.current?.(data);
    });
    const frame = window.requestAnimationFrame(() => {
      fitAddon.fit();
    });
    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(host);

    return () => {
      cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      dataListener.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      terminal.dispose();
    };
  }, []);

  useEffect(() => {
    const terminal = terminalRef.current;
    if (!terminal) {
      return;
    }

    terminal.reset();
    if (snapshot.title) {
      terminal.writeln(`\u001b[1m${snapshot.title}\u001b[0m`);
    }
    if (snapshot.working_directory) {
      terminal.writeln(snapshot.working_directory);
    }
    if (snapshot.child_process_id) {
      terminal.writeln(`pid ${snapshot.child_process_id}`);
    }

    const lines = snapshot.visible_text.split("\n");
    if (lines.length > 0) {
      terminal.writeln("");
      terminal.writeln(lines.join("\n"));
    }

    fitAddonRef.current?.fit();
  }, [
    snapshot.child_process_id,
    snapshot.columns,
    snapshot.id,
    snapshot.rows,
    snapshot.title,
    snapshot.visible_text,
    snapshot.working_directory,
  ]);

  return (
    <section className="terminalSurface">
      <header className="terminalSurfaceHeader">
        <div>
          <p className="eyebrow">Terminal</p>
          <h3>{snapshot.title || "Untitled session"}</h3>
        </div>
        <div className="terminalSurfaceMeta">
          <span>{snapshot.working_directory || "No working directory"}</span>
          <span>{snapshot.is_running ? "Running" : "Exited"}</span>
        </div>
      </header>

      <div className="terminalSurfaceViewport">
        <div ref={hostRef} className="terminalSurfaceHost" />
      </div>

      <footer className="terminalSurfaceFooter">
        <div className="terminalSurfaceStats">
          <span>
            {snapshot.columns} x {snapshot.rows}
          </span>
          <span>{snapshot.bell_count} bells</span>
        </div>
        <div className="terminalSurfaceSearch">
          <input
            className="terminalSurfaceSearchInput"
            type="search"
            value={searchText}
            onChange={(event) => setSearchText(event.target.value)}
            placeholder="Find in terminal"
            aria-label="Search terminal contents"
          />
          <button type="button" className="ghostButton" disabled>
            Prev
          </button>
          <button type="button" className="ghostButton" disabled>
            Next
          </button>
          <span>{searchCount} matches</span>
        </div>
      </footer>
    </section>
  );
}

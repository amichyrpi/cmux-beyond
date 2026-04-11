import { useEffect, useState } from "react";
import type { DragEvent } from "react";

import {
  backBrowserWindow,
  ensureBrowserWindow,
  ensureTerminalSession,
  forwardBrowserWindow,
  getBrowserSnapshot,
  getTerminalSnapshot,
  reloadBrowserWindow,
  sendTerminalInput,
} from "../bridge";
import { BrowserPanel } from "../browser";
import type { BrowserSessionSnapshot } from "../browser";
import { TerminalSurface } from "../terminal";
import type { TerminalSnapshot } from "../terminal";
import type { ExternalPaneNode } from "./bonsplit-types";
import { readTabDragPayload } from "./drag";
import { TabBar } from "./TabBar";

type DropZone = "center" | "left" | "right" | "top" | "bottom";

export interface PaneHostProps {
  pane: ExternalPaneNode;
  workspaceDirectory: string;
  focused: boolean;
  onFocusPane: () => void;
  onAddTab: () => void;
  onAddBrowserTab: () => void;
  onClosePane: () => void;
  onCloseTab: (tabId: string) => void;
  onSelectTab: (tabId: string) => void;
  onMoveTab: (tabId: string, sourcePaneId: string, targetPaneId: string, index: number) => void;
  onSplitDrop: (
    tabId: string,
    sourcePaneId: string,
    targetPaneId: string,
    zone: Exclude<DropZone, "center">,
  ) => void;
}

const EDGE_RATIO = 0.24;

function zoneFromPoint(event: DragEvent<HTMLElement>, rect: DOMRect): DropZone {
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  const xRatio = rect.width > 0 ? x / rect.width : 0.5;
  const yRatio = rect.height > 0 ? y / rect.height : 0.5;

  if (xRatio < EDGE_RATIO) {
    return "left";
  }
  if (xRatio > 1 - EDGE_RATIO) {
    return "right";
  }
  if (yRatio < EDGE_RATIO) {
    return "top";
  }
  if (yRatio > 1 - EDGE_RATIO) {
    return "bottom";
  }
  return "center";
}

const makeTerminalFallbackSnapshot = (
  paneId: string,
  workspaceDirectory: string,
  title: string,
): TerminalSnapshot => ({
  id: paneId,
  working_directory: workspaceDirectory,
  columns: 80,
  rows: 24,
  title,
  bell_count: 0,
  is_running: true,
  child_process_id: null,
  visible_text: [
    `$ cd ${workspaceDirectory}`,
    `$ ${title || "terminal"}`,
    "",
    "Phase 6 terminal scaffold is live.",
  ].join("\n"),
});

const makeBrowserFallbackSnapshot = (
  paneId: string,
  title: string,
): BrowserSessionSnapshot => ({
  id: paneId,
  window_label: `browser-${paneId}`,
  workspace_id: null,
  profile_id: null,
  current_url: null,
  title,
  page_zoom: 1,
  developer_tools_visible: false,
  should_render_webview: true,
  back_history: [],
  forward_history: [],
});

export function PaneHost({
  pane,
  workspaceDirectory,
  focused,
  onFocusPane,
  onAddTab,
  onAddBrowserTab,
  onClosePane,
  onCloseTab,
  onSelectTab,
  onMoveTab,
  onSplitDrop,
}: PaneHostProps) {
  const [dropZone, setDropZone] = useState<DropZone | null>(null);
  const [terminalSnapshot, setTerminalSnapshot] = useState<TerminalSnapshot | null>(null);
  const [browserSnapshot, setBrowserSnapshot] = useState<BrowserSessionSnapshot | null>(null);

  const selectedTab = pane.tabs.find((tab) => tab.id === pane.selected_tab_id) ?? pane.tabs[0] ?? null;
  const selectedKind = selectedTab?.kind?.toLowerCase() ?? "terminal";
  const terminalSessionKey = `terminal-${pane.id}`;
  const browserWindowLabel = `browser-${pane.id}`;
  const title = selectedTab?.title ?? (selectedKind === "browser" ? "Browser" : "Terminal");

  useEffect(() => {
    let active = true;

    if (selectedKind !== "terminal") {
      setTerminalSnapshot(null);
      return () => {
        active = false;
      };
    }

    void ensureTerminalSession(terminalSessionKey, workspaceDirectory)
      .then((snapshot) => {
        if (active) {
          setTerminalSnapshot(snapshot);
        }
      })
      .catch(() => {
        if (active) {
          setTerminalSnapshot(makeTerminalFallbackSnapshot(pane.id, workspaceDirectory, title));
        }
      });

    const interval = window.setInterval(() => {
      void getTerminalSnapshot(terminalSessionKey)
        .then((snapshot) => {
          if (active) {
            setTerminalSnapshot(snapshot);
          }
        })
        .catch(() => {
          if (active) {
            setTerminalSnapshot(makeTerminalFallbackSnapshot(pane.id, workspaceDirectory, title));
          }
        });
    }, 500);

    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [pane.id, selectedKind, terminalSessionKey, title, workspaceDirectory]);

  useEffect(() => {
    let active = true;

    if (selectedKind !== "browser") {
      setBrowserSnapshot(null);
      return () => {
        active = false;
      };
    }

    void ensureBrowserWindow(browserWindowLabel, undefined, title)
      .then((snapshot) => {
        if (active) {
          setBrowserSnapshot(snapshot);
        }
      })
      .catch(() => {
        if (active) {
          setBrowserSnapshot(makeBrowserFallbackSnapshot(pane.id, title));
        }
      });

    const interval = window.setInterval(() => {
      void getBrowserSnapshot(browserWindowLabel)
        .then((snapshot) => {
          if (active) {
            setBrowserSnapshot(snapshot);
          }
        })
        .catch(() => {
          if (active) {
            setBrowserSnapshot(makeBrowserFallbackSnapshot(pane.id, title));
          }
        });
    }, 1000);

    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [browserWindowLabel, pane.id, selectedKind, title]);

  return (
    <section
      className={`paneHost${focused ? " is-focused" : ""}`}
      onClick={onFocusPane}
      onDragOver={(event) => {
        const payload = readTabDragPayload(event);
        if (!payload) {
          return;
        }
        event.preventDefault();
        setDropZone(zoneFromPoint(event, event.currentTarget.getBoundingClientRect()));
      }}
      onDragLeave={() => setDropZone(null)}
      onDrop={(event) => {
        const payload = readTabDragPayload(event);
        if (!payload) {
          return;
        }
        event.preventDefault();
        const zone = zoneFromPoint(event, event.currentTarget.getBoundingClientRect());
        if (zone === "center") {
          onMoveTab(payload.tab_id, payload.source_pane_id, pane.id, pane.tabs.length);
        } else {
          onSplitDrop(payload.tab_id, payload.source_pane_id, pane.id, zone);
        }
        setDropZone(null);
      }}
    >
      <header className="paneHeader">
        <div className="paneHeaderCopy">
          <span className="paneHeaderEyebrow">Pane</span>
          <strong>{pane.id.slice(0, 8)}</strong>
        </div>
        <div className="paneHeaderActions">
          <button type="button" className="iconButton" onClick={onAddTab} title="Add terminal tab">
            +
          </button>
          <button type="button" className="iconButton" onClick={onAddBrowserTab} title="Add browser tab">
            B
          </button>
          <button type="button" className="iconButton" onClick={onClosePane} title="Close pane">
            x
          </button>
        </div>
      </header>

      <TabBar
        paneId={pane.id}
        tabs={pane.tabs}
        selectedTabId={pane.selected_tab_id}
        onSelectTab={onSelectTab}
        onAddTab={onAddTab}
        onCloseTab={onCloseTab}
        onMoveTab={onMoveTab}
      />

      <div className="paneBody">
        {pane.tabs.length === 0 ? (
          <div className="paneEmptyState">
            <p>Drop a tab here or use the + and B buttons.</p>
          </div>
        ) : selectedKind === "browser" ? (
          <BrowserPanel
            snapshot={browserSnapshot ?? makeBrowserFallbackSnapshot(pane.id, title)}
            onOpenUrl={async ({ url, target_title }) => {
              const snapshot = await ensureBrowserWindow(browserWindowLabel, url, target_title ?? title);
              setBrowserSnapshot(snapshot);
            }}
            onBack={
              browserSnapshot
                ? async () => {
                    await backBrowserWindow(browserSnapshot.id);
                    setBrowserSnapshot(await getBrowserSnapshot(browserWindowLabel));
                  }
                : undefined
            }
            onForward={
              browserSnapshot
                ? async () => {
                    await forwardBrowserWindow(browserSnapshot.id);
                    setBrowserSnapshot(await getBrowserSnapshot(browserWindowLabel));
                  }
                : undefined
            }
            onReload={
              browserSnapshot
                ? async () => {
                    await reloadBrowserWindow(browserSnapshot.id);
                    setBrowserSnapshot(await getBrowserSnapshot(browserWindowLabel));
                  }
                : undefined
            }
          />
        ) : (
          <TerminalSurface
            snapshot={terminalSnapshot ?? makeTerminalFallbackSnapshot(pane.id, workspaceDirectory, title)}
            onInput={async (input) => {
              await sendTerminalInput(terminalSessionKey, input);
              setTerminalSnapshot(await getTerminalSnapshot(terminalSessionKey));
            }}
          />
        )}

        {dropZone && dropZone !== "center" ? (
          <div className={`paneDropOverlay zone-${dropZone}`}>
            <span>
              {dropZone === "left" || dropZone === "right" ? "Split horizontally" : "Split vertically"}
            </span>
          </div>
        ) : null}
      </div>
    </section>
  );
}

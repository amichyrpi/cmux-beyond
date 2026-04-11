import { useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import type { DragEvent } from "react";
import {
  addTab,
  clearNotifications,
  closePane,
  closeTab,
  closeWorkspace,
  createWorkspace,
  getNotificationState,
  getState,
  moveTab,
  markNotificationRead,
  pushNotification,
  reorderWorkspace,
  scanPorts,
  selectPane,
  selectTab,
  selectWorkspace,
  setDividerPosition,
  splitPane,
} from "./bridge";
import type { AppSnapshot, WorkspaceSnapshot } from "./bonsplit";
import { SplitContainer } from "./bonsplit";
import { SidebarPanel } from "./sidebar";
import type { NotificationAppSnapshot } from "./notifications/notification-types";
import type { PortScanSnapshot } from "./ports/ports-types";

const WORKSPACE_DRAG_MIME = "application/x-cmux-workspace";
const STATE_EVENT = "cmux:state";
const NOTIFICATION_EVENT = "cmux:notifications";

type SidebarTab = "workspace" | "notifications" | "ports";

function workspaceLabel(workspace: WorkspaceSnapshot, index: number) {
  return workspace.custom_title || workspace.title || `Workspace ${index + 1}`;
}

function zoneToSplit(zone: "left" | "right" | "top" | "bottom") {
  return zone === "left" || zone === "right"
    ? { orientation: "horizontal" as const, insert_first: zone === "left" }
    : { orientation: "vertical" as const, insert_first: zone === "top" };
}

async function updateSnapshot(
  action: Promise<AppSnapshot>,
  setSnapshot: Dispatch<SetStateAction<AppSnapshot | null>>,
) {
  const next = await action;
  setSnapshot((current) => (current?.revision === next.revision ? current : next));
  return next;
}

export function App() {
  const [version, setVersion] = useState<string>("...");
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [notifications, setNotifications] = useState<NotificationAppSnapshot | null>(null);
  const [ports, setPorts] = useState<PortScanSnapshot | null>(null);
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>("workspace");

  useEffect(() => {
    let active = true;

    void getVersion()
      .then((value) => {
        if (active) {
          setVersion(value);
        }
      })
      .catch(() => {
        if (active) {
          setVersion("unknown");
        }
      });

    void getState()
      .then((state) => {
        if (active) {
          setSnapshot(state);
        }
      })
      .catch(() => {
        if (active) {
          setSnapshot(null);
        }
      });

    void getNotificationState()
      .then((state) => {
        if (active) {
          setNotifications(state);
        }
      })
      .catch(() => {
        if (active) {
          setNotifications(null);
        }
      });

    void scanPorts()
      .then((state) => {
        if (active) {
          setPorts(state);
        }
      })
      .catch(() => {
        if (active) {
          setPorts(null);
        }
      });

    const unlistenStatePromise = listen<AppSnapshot>(STATE_EVENT, (event) => {
      const nextSnapshot = event.payload;
      setSnapshot((current) =>
        current?.revision === nextSnapshot.revision ? current : nextSnapshot,
      );
    });

    const unlistenNotificationsPromise = listen<NotificationAppSnapshot>(NOTIFICATION_EVENT, (event) => {
      const nextNotifications = event.payload;
      setNotifications((current) =>
        current?.revision === nextNotifications.revision ? current : nextNotifications,
      );
    });

    return () => {
      active = false;
      void unlistenStatePromise.then((unlisten) => unlisten());
      void unlistenNotificationsPromise.then((unlisten) => unlisten());
    };
  }, []);

  const tabManager = snapshot?.tab_manager;
  const workspaces = tabManager?.workspaces ?? [];
  const selectedWorkspaceId = tabManager?.selected_workspace_id ?? null;
  const selectedWorkspace =
    workspaces.find((workspace) => workspace.id === selectedWorkspaceId) ?? workspaces[0] ?? null;

  async function refreshAfter(action: Promise<AppSnapshot>) {
    return updateSnapshot(action, setSnapshot);
  }

  async function handleCreateWorkspace() {
    const label = `Workspace ${workspaces.length + 1}`;
    await refreshAfter(createWorkspace(label));
  }

  async function handleSelectWorkspace(workspaceId: string) {
    await refreshAfter(selectWorkspace(workspaceId));
  }

  async function handleCloseWorkspace(workspaceId: string) {
    await refreshAfter(closeWorkspace(workspaceId));
  }

  async function handleReorderWorkspace(workspaceId: string, targetIndex: number) {
    await refreshAfter(reorderWorkspace(workspaceId, targetIndex));
  }

  async function handleFocusPane(paneId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(selectPane(selectedWorkspace.id, paneId));
  }

  async function handleAddTerminalTab(paneId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(addTab(selectedWorkspace.id, paneId, "Untitled", "terminal"));
  }

  async function handleAddBrowserTab(paneId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(addTab(selectedWorkspace.id, paneId, "Browser", "browser"));
  }

  async function handleClosePane(paneId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(closePane(selectedWorkspace.id, paneId));
  }

  async function handleCloseTab(paneId: string, tabId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(closeTab(selectedWorkspace.id, paneId, tabId));
  }

  async function handleMoveTab(
    tabId: string,
    sourcePaneId: string,
    targetPaneId: string,
    index: number,
  ) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(moveTab(selectedWorkspace.id, tabId, sourcePaneId, targetPaneId, index));
  }

  async function handleSplitDrop(
    tabId: string,
    sourcePaneId: string,
    targetPaneId: string,
    zone: "left" | "right" | "top" | "bottom",
  ) {
    if (!selectedWorkspace) {
      return;
    }

    const { orientation, insert_first } = zoneToSplit(zone);
    const splitResult = await refreshAfter(
      splitPane(selectedWorkspace.id, targetPaneId, orientation, undefined, insert_first),
    );

    const nextWorkspace = splitResult.tab_manager.workspaces.find(
      (workspace) => workspace.id === selectedWorkspace.id,
    );
    const newPaneId = nextWorkspace?.focused_pane_id ?? null;
    if (!newPaneId) {
      return;
    }
    await refreshAfter(moveTab(selectedWorkspace.id, tabId, sourcePaneId, newPaneId, 0));
  }

  async function handleDividerUpdate(splitId: string, position: number) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(setDividerPosition(selectedWorkspace.id, splitId, position));
  }

  async function handleSelectTab(paneId: string, tabId: string) {
    if (!selectedWorkspace) {
      return;
    }
    await refreshAfter(selectTab(selectedWorkspace.id, paneId, tabId));
  }

  async function handleRefreshPorts() {
    setPorts(await scanPorts());
  }

  async function handlePushNotification() {
    setNotifications(
      await pushNotification(
        "Phase 8",
        "Sidebar notifications are wired through Tauri.",
        "info",
      ),
    );
  }

  async function handleClearNotifications() {
    setNotifications(await clearNotifications());
  }

  async function handleMarkNotificationRead(notificationId: string) {
    setNotifications(await markNotificationRead(notificationId));
  }

  function handleWorkspaceDragStart(event: DragEvent<HTMLElement>, workspaceId: string) {
    event.dataTransfer.setData(WORKSPACE_DRAG_MIME, workspaceId);
    event.dataTransfer.effectAllowed = "move";
  }

  async function handleWorkspaceDrop(event: DragEvent<HTMLElement>, targetIndex: number) {
    event.preventDefault();
    const workspaceId = event.dataTransfer.getData(WORKSPACE_DRAG_MIME);
    if (!workspaceId) {
      return;
    }
    await handleReorderWorkspace(workspaceId, targetIndex);
  }

  return (
    <div className="shell">
      <header className="chrome">
        <div className="brandBlock">
          <p className="eyebrow">cmux</p>
          <h1>Rust workspace shell</h1>
          <p className="lede">Phase 5 shell with Phase 6 terminal, Phase 7 browser, and Phase 8 sidebar.</p>
        </div>

        <div className="chromeActions">
          <div className="workspaceStrip" role="tablist" aria-label="Workspaces">
            {workspaces.map((workspace, index) => {
              const isSelected = workspace.id === selectedWorkspace?.id;
              return (
                <button
                  key={workspace.id}
                  type="button"
                  className={`workspaceChip${isSelected ? " is-selected" : ""}`}
                  role="tab"
                  draggable
                  aria-selected={isSelected}
                  onClick={() => void handleSelectWorkspace(workspace.id)}
                  onDragStart={(event) => handleWorkspaceDragStart(event, workspace.id)}
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={(event) => {
                    void handleWorkspaceDrop(event, index);
                  }}
                >
                  <span>{workspaceLabel(workspace, index)}</span>
                  <span
                    className="workspaceChipClose"
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleCloseWorkspace(workspace.id);
                    }}
                    aria-label={`Close ${workspaceLabel(workspace, index)}`}
                  >
                    x
                  </span>
                </button>
              );
            })}
          </div>

          <button type="button" className="primaryButton" onClick={() => void handleCreateWorkspace()}>
            New workspace
          </button>
          <div className="versionBadge">v{version}</div>
        </div>
      </header>

      <main className="stage">
        {selectedWorkspace ? (
          <div className="workspaceSurface">
            <section className="workspaceMain">
              <section className="workspaceBanner">
                <div>
                  <p className="eyebrow">Selected</p>
                  <h2>{workspaceLabel(selectedWorkspace, 0)}</h2>
                </div>
                <div className="workspaceBannerMeta">
                  <span>{selectedWorkspace.current_directory}</span>
                  <span>Revision {snapshot?.revision ?? 0}</span>
                </div>
              </section>

              <SplitContainer
                workspace={selectedWorkspace}
                onFocusPane={(paneId) => void handleFocusPane(paneId)}
                onAddTab={(paneId) => void handleAddTerminalTab(paneId)}
                onAddBrowserTab={(paneId) => void handleAddBrowserTab(paneId)}
                onClosePane={(paneId) => void handleClosePane(paneId)}
                onCloseTab={(paneId, tabId) => void handleCloseTab(paneId, tabId)}
                onSelectTab={(paneId, tabId) => void handleSelectTab(paneId, tabId)}
                onMoveTab={(tabId, sourcePaneId, targetPaneId, index) =>
                  void handleMoveTab(tabId, sourcePaneId, targetPaneId, index)
                }
                onSplitDrop={(tabId, sourcePaneId, targetPaneId, zone) =>
                  void handleSplitDrop(tabId, sourcePaneId, targetPaneId, zone)
                }
                onSetDividerPosition={(splitId, position) =>
                  void handleDividerUpdate(splitId, position)
                }
              />
            </section>

            <SidebarPanel
              workspace={selectedWorkspace}
              notifications={notifications}
              ports={ports}
              activeTab={sidebarTab}
              onChangeTab={setSidebarTab}
              onRefreshPorts={() => void handleRefreshPorts()}
              onPushNotification={() => void handlePushNotification()}
              onClearNotifications={() => void handleClearNotifications()}
              onMarkNotificationRead={(notificationId) => void handleMarkNotificationRead(notificationId)}
            />
          </div>
        ) : (
          <section className="emptyStage">
            <h2>No workspace loaded</h2>
            <p>Initialize the Tauri state or create a workspace to begin.</p>
          </section>
        )}
      </main>
    </div>
  );
}

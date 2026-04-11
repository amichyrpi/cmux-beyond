import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, PaneId, WorkspaceId } from "./bonsplit/bonsplit-types";
import type { BrowserAppSnapshot, BrowserPopupRequest, BrowserSessionSnapshot } from "./browser";
import type { NotificationAppSnapshot, NotificationLevel } from "./notifications/notification-types";
import type { PortScanSnapshot } from "./ports/ports-types";
import type { TerminalAppSnapshot, TerminalSearchResult, TerminalSnapshot } from "./terminal";

export async function getState(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_state");
}

export async function createWorkspace(title?: string): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_create", { title });
}

export async function selectWorkspace(workspace_id: WorkspaceId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_select", { workspace_id });
}

export async function closeWorkspace(workspace_id: WorkspaceId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_close", { workspace_id });
}

export async function reorderWorkspace(
  workspace_id: WorkspaceId,
  target_index: number,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_reorder", { workspace_id, target_index });
}

export async function addTab(
  workspace_id: WorkspaceId,
  pane_id: PaneId,
  title?: string,
  kind?: string,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_add_tab", { workspace_id, pane_id, title, kind });
}

export async function splitPane(
  workspace_id: WorkspaceId,
  pane_id: PaneId,
  orientation: "horizontal" | "vertical",
  title?: string,
  insert_first?: boolean,
  kind?: string,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_split_pane", {
    workspace_id,
    pane_id,
    orientation,
    title,
    insert_first,
    kind,
  });
}

export async function closePane(workspace_id: WorkspaceId, pane_id: PaneId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_close_pane", { workspace_id, pane_id });
}

export async function closeTab(
  workspace_id: WorkspaceId,
  pane_id: PaneId,
  tab_id: string,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_close_tab", { workspace_id, pane_id, tab_id });
}

export async function moveTab(
  workspace_id: WorkspaceId,
  tab_id: string,
  source_pane_id: PaneId,
  target_pane_id: PaneId,
  index?: number,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_move_tab", {
    workspace_id,
    tab_id,
    source_pane_id,
    target_pane_id,
    index,
  });
}

export async function selectPane(workspace_id: WorkspaceId, pane_id: PaneId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_select_pane", { workspace_id, pane_id });
}

export async function selectTab(
  workspace_id: WorkspaceId,
  pane_id: PaneId,
  tab_id: string,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_select_tab", { workspace_id, pane_id, tab_id });
}

export async function setDividerPosition(
  workspace_id: WorkspaceId,
  split_id: string,
  position: number,
): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_set_divider_position", {
    workspace_id,
    split_id,
    position,
  });
}

export async function selectNextTab(workspace_id: WorkspaceId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_select_next_tab", { workspace_id });
}

export async function selectPreviousTab(workspace_id: WorkspaceId): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("workspace_select_previous_tab", { workspace_id });
}

export async function getBrowserState(): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_state");
}

export async function ensureBrowserWindow(
  window_label: string,
  url?: string,
  title?: string,
): Promise<BrowserSessionSnapshot> {
  return invoke<BrowserSessionSnapshot>("browser_ensure", { window_label, url, title });
}

export async function getBrowserSnapshot(window_label: string): Promise<BrowserSessionSnapshot> {
  return invoke<BrowserSessionSnapshot>("browser_snapshot_by_label", { window_label });
}

export async function openBrowserWindow(request: BrowserPopupRequest): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_open", { request });
}

export async function navigateBrowserWindow(
  session_id: string,
  url: string,
): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_navigate", { session_id, url });
}

export async function reloadBrowserWindow(session_id: string): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_reload", { session_id });
}

export async function backBrowserWindow(session_id: string): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_back", { session_id });
}

export async function forwardBrowserWindow(session_id: string): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_forward", { session_id });
}

export async function closeBrowserWindow(session_id: string): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_close", { session_id });
}

export async function setBrowserWindowTitle(
  session_id: string,
  title?: string,
): Promise<BrowserAppSnapshot> {
  return invoke<BrowserAppSnapshot>("browser_set_title", { session_id, title });
}

export async function getTerminalState(): Promise<TerminalAppSnapshot> {
  return invoke<TerminalAppSnapshot>("terminal_state");
}

export async function ensureTerminalSession(
  session_key: string,
  working_directory?: string,
): Promise<TerminalSnapshot> {
  return invoke<TerminalSnapshot>("terminal_ensure", { session_key, working_directory });
}

export async function getTerminalSnapshot(session_key: string): Promise<TerminalSnapshot> {
  return invoke<TerminalSnapshot>("terminal_snapshot", { session_key });
}

export async function sendTerminalInput(
  session_key: string,
  input: string,
): Promise<TerminalSnapshot> {
  return invoke<TerminalSnapshot>("terminal_input", { session_key, input });
}

export async function resizeTerminalSession(
  session_key: string,
  columns: number,
  rows: number,
): Promise<TerminalSnapshot> {
  return invoke<TerminalSnapshot>("terminal_resize", { session_key, columns, rows });
}

export async function searchTerminalSession(
  session_key: string,
  query: string,
): Promise<TerminalSearchResult> {
  return invoke<TerminalSearchResult>("terminal_search", { session_key, query });
}

export async function getNotificationState(): Promise<NotificationAppSnapshot> {
  return invoke<NotificationAppSnapshot>("notifications_state");
}

export async function pushNotification(
  title: string,
  body: string,
  level?: NotificationLevel,
): Promise<NotificationAppSnapshot> {
  return invoke<NotificationAppSnapshot>("notifications_push", { title, body, level });
}

export async function markNotificationRead(notification_id: string): Promise<NotificationAppSnapshot> {
  return invoke<NotificationAppSnapshot>("notifications_mark_read", { notification_id });
}

export async function clearNotifications(): Promise<NotificationAppSnapshot> {
  return invoke<NotificationAppSnapshot>("notifications_clear");
}

export async function scanPorts(): Promise<PortScanSnapshot> {
  return invoke<PortScanSnapshot>("ports_scan");
}

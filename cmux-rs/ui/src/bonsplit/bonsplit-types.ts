export type TabId = string;
export type PaneId = string;
export type WorkspaceId = string;
export type SplitOrientation = "horizontal" | "vertical";

export interface TabItem {
  id: TabId;
  title: string;
  has_custom_title: boolean;
  icon?: string | null;
  kind?: string | null;
  is_dirty: boolean;
  shows_notification_badge: boolean;
  is_loading: boolean;
  is_pinned: boolean;
}

export interface PaneState {
  id: PaneId;
  tabs: TabItem[];
  selected_tab_id?: TabId | null;
}

export interface SplitState {
  id: string;
  orientation: SplitOrientation;
  first: ExternalTreeNode;
  second: ExternalTreeNode;
  divider_position: number;
}

export interface PaneGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ExternalPaneNode {
  id: PaneId;
  frame: PaneGeometry;
  tabs: TabItem[];
  selected_tab_id?: TabId | null;
}

export interface ExternalSplitNode {
  id: string;
  orientation: SplitOrientation;
  divider_position: number;
  first: ExternalTreeNode;
  second: ExternalTreeNode;
}

export type ExternalTreeNode =
  | { type: "pane"; pane: ExternalPaneNode }
  | { type: "split"; split: ExternalSplitNode };

export interface WorkspaceSnapshot {
  id: WorkspaceId;
  title: string;
  custom_title?: string | null;
  custom_description?: string | null;
  custom_color?: string | null;
  is_pinned: boolean;
  current_directory: string;
  preferred_browser_profile_id?: string | null;
  port_ordinal: number;
  focused_pane_id?: PaneId | null;
  zoomed_pane_id?: PaneId | null;
  tree: ExternalTreeNode;
}

export interface TabManagerSnapshot {
  selected_workspace_id?: WorkspaceId | null;
  workspaces: WorkspaceSnapshot[];
}

export interface AppSnapshot {
  revision: number;
  tab_manager: TabManagerSnapshot;
}

export function isPaneNode(node: ExternalTreeNode): node is { type: "pane"; pane: ExternalPaneNode } {
  return node.type === "pane";
}

export function isSplitNode(node: ExternalTreeNode): node is { type: "split"; split: ExternalSplitNode } {
  return node.type === "split";
}

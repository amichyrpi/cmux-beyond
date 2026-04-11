export interface BrowserHistoryEntry {
  url: string;
  title?: string | null;
}

export interface BrowserSessionSnapshot {
  id: string;
  window_label: string;
  workspace_id?: string | null;
  profile_id?: string | null;
  current_url?: string | null;
  title?: string | null;
  page_zoom: number;
  developer_tools_visible: boolean;
  should_render_webview: boolean;
  back_history: BrowserHistoryEntry[];
  forward_history: BrowserHistoryEntry[];
}

export interface BrowserPopupRequest {
  url: string;
  opener_window_label?: string | null;
  target_title?: string | null;
}

export interface BrowserPanelProps {
  snapshot: BrowserSessionSnapshot;
  onOpenUrl?: (request: BrowserPopupRequest) => void | Promise<void>;
  onBack?: () => void | Promise<void>;
  onForward?: () => void | Promise<void>;
  onReload?: () => void | Promise<void>;
}

export interface BrowserAppSnapshot {
  revision: number;
  sessions: BrowserSessionSnapshot[];
}

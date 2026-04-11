export interface TerminalSnapshot {
  id: string;
  working_directory?: string | null;
  columns: number;
  rows: number;
  title?: string | null;
  bell_count: number;
  is_running: boolean;
  child_process_id?: number | null;
  visible_text: string;
}

export interface TerminalSearchMatch {
  line_index: number;
  column_index: number;
  matched_text: string;
}

export interface TerminalSearchResult {
  query: string;
  matches: TerminalSearchMatch[];
}

export interface TerminalAppSnapshot {
  revision: number;
  sessions: TerminalSnapshot[];
}

export interface TerminalSurfaceProps {
  snapshot: TerminalSnapshot;
  searchQuery?: string;
  onInput?: (input: string) => void | Promise<void>;
}

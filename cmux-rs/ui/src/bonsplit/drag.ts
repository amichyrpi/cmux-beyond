import type { TabId, PaneId } from "./bonsplit-types";
import type { DragEvent } from "react";

export const TAB_DRAG_MIME = "application/x-cmux-tab";

export interface TabDragPayload {
  tab_id: TabId;
  source_pane_id: PaneId;
  source_index: number;
  title: string;
}

export function writeTabDragPayload(
  event: DragEvent<HTMLElement>,
  payload: TabDragPayload,
) {
  event.dataTransfer.setData(TAB_DRAG_MIME, JSON.stringify(payload));
  event.dataTransfer.effectAllowed = "move";
}

export function readTabDragPayload(event: DragEvent<HTMLElement>): TabDragPayload | null {
  const raw = event.dataTransfer.getData(TAB_DRAG_MIME);
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as TabDragPayload;
  } catch {
    return null;
  }
}

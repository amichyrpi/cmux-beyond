import type { TabItem } from "./bonsplit-types";
import { readTabDragPayload, writeTabDragPayload } from "./drag";

export interface TabBarProps {
  paneId: string;
  tabs: TabItem[];
  selectedTabId?: string | null;
  onSelectTab: (tabId: string) => void;
  onAddTab: () => void;
  onCloseTab: (tabId: string) => void;
  onMoveTab: (tabId: string, sourcePaneId: string, targetPaneId: string, index: number) => void;
}

export function TabBar({
  paneId,
  tabs,
  selectedTabId,
  onSelectTab,
  onAddTab,
  onCloseTab,
  onMoveTab,
}: TabBarProps) {
  return (
    <div className="tabBar" role="tablist" aria-label="Pane tabs">
      {tabs.map((tab, index) => {
        const label = tab.title || `Tab ${index + 1}`;
        const isSelected = selectedTabId === tab.id;
        const kind = tab.kind ? tab.kind.toLowerCase() : null;
        return (
          <button
            key={tab.id}
            type="button"
            className={`tabChip${isSelected ? " is-selected" : ""}`}
            role="tab"
            aria-selected={isSelected}
            draggable
            onClick={() => onSelectTab(tab.id)}
            onDragStart={(event) =>
              writeTabDragPayload(event, {
                tab_id: tab.id,
                source_pane_id: paneId,
                source_index: index,
                title: tab.title,
              })
            }
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const payload = readTabDragPayload(event);
              if (!payload) {
                return;
              }
              onMoveTab(payload.tab_id, payload.source_pane_id, paneId, index);
            }}
          >
            {kind ? <span className="tabChipKind">{kind}</span> : null}
            <span className="tabChipLabel">{label}</span>
            <span
              className="tabChipClose"
              aria-label={`Close ${label}`}
              onClick={(event) => {
                event.stopPropagation();
                onCloseTab(tab.id);
              }}
            >
              x
            </span>
          </button>
        );
      })}

      <button
        type="button"
        className="tabChip tabChipGhost"
        onClick={onAddTab}
        aria-label="Append tab placeholder"
        title="Drop a tab here to append it"
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => {
          event.preventDefault();
          const payload = readTabDragPayload(event);
          if (!payload) {
            return;
          }
          onMoveTab(payload.tab_id, payload.source_pane_id, paneId, tabs.length);
        }}
      >
        +
      </button>
    </div>
  );
}

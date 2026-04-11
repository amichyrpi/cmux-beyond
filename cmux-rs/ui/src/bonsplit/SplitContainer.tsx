import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import type { ExternalTreeNode, WorkspaceSnapshot } from "./bonsplit-types";
import { isPaneNode } from "./bonsplit-types";
import { PaneHost } from "./PaneHost";

export interface SplitContainerProps {
  workspace: WorkspaceSnapshot;
  onFocusPane: (paneId: string) => void;
  onAddTab: (paneId: string) => void;
  onAddBrowserTab: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onCloseTab: (paneId: string, tabId: string) => void;
  onSelectTab: (paneId: string, tabId: string) => void;
  onMoveTab: (tabId: string, sourcePaneId: string, targetPaneId: string, index: number) => void;
  onSplitDrop: (
    tabId: string,
    sourcePaneId: string,
    targetPaneId: string,
    zone: "left" | "right" | "top" | "bottom",
  ) => void;
  onSetDividerPosition: (splitId: string, position: number) => void;
}

export function SplitContainer({
  workspace,
  onFocusPane,
  onAddTab,
  onAddBrowserTab,
  onClosePane,
  onCloseTab,
  onSelectTab,
  onMoveTab,
  onSplitDrop,
  onSetDividerPosition,
}: SplitContainerProps) {
  return <div className="splitContainer">{renderNode(workspace.tree)}</div>;

  function renderNode(node: ExternalTreeNode): JSX.Element {
    if (isPaneNode(node)) {
      const focused = workspace.focused_pane_id === node.pane.id;
      return (
        <PaneHost
          pane={node.pane}
          workspaceDirectory={workspace.current_directory}
          focused={focused}
          onFocusPane={() => onFocusPane(node.pane.id)}
          onAddTab={() => onAddTab(node.pane.id)}
          onAddBrowserTab={() => onAddBrowserTab(node.pane.id)}
          onClosePane={() => onClosePane(node.pane.id)}
          onCloseTab={(tabId) => onCloseTab(node.pane.id, tabId)}
          onSelectTab={(tabId) => onSelectTab(node.pane.id, tabId)}
          onMoveTab={onMoveTab}
          onSplitDrop={(tabId, sourcePaneId, targetPaneId, zone) =>
            onSplitDrop(tabId, sourcePaneId, targetPaneId, zone)
          }
        />
      );
    }

    const split = node.split;
    const direction = split.orientation === "horizontal" ? "horizontal" : "vertical";
    const firstDefault = Math.max(5, Math.min(95, split.divider_position * 100));
    const secondDefault = 100 - firstDefault;

    return (
      <PanelGroup
        className="splitGroup"
        direction={direction}
        onLayout={(sizes) => {
          if (sizes.length >= 2) {
            const total = sizes[0] + sizes[1];
            if (total > 0) {
              onSetDividerPosition(split.id, sizes[0] / total);
            }
          }
        }}
      >
        <Panel defaultSize={firstDefault} minSize={12}>
          {renderNode(split.first)}
        </Panel>
        <PanelResizeHandle className="splitResizeHandle" />
        <Panel defaultSize={secondDefault} minSize={12}>
          {renderNode(split.second)}
        </Panel>
      </PanelGroup>
    );
  }
}

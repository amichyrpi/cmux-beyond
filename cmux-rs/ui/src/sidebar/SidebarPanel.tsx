import type { WorkspaceSnapshot } from "../bonsplit";
import type { NotificationAppSnapshot, NotificationEntry } from "../notifications/notification-types";
import type { PortScanSnapshot } from "../ports/ports-types";

export type SidebarTab = "workspace" | "notifications" | "ports";

export interface SidebarPanelProps {
  workspace: WorkspaceSnapshot | null;
  notifications: NotificationAppSnapshot | null;
  ports: PortScanSnapshot | null;
  activeTab: SidebarTab;
  onChangeTab: (tab: SidebarTab) => void;
  onRefreshPorts: () => void;
  onPushNotification: () => void;
  onClearNotifications: () => void;
  onMarkNotificationRead?: (notificationId: string) => void | Promise<void>;
}

function notificationLabel(entry: NotificationEntry) {
  return entry.title || entry.body || "Notification";
}

export function SidebarPanel({
  workspace,
  notifications,
  ports,
  activeTab,
  onChangeTab,
  onRefreshPorts,
  onPushNotification,
  onClearNotifications,
  onMarkNotificationRead,
}: SidebarPanelProps) {
  return (
    <aside className="sidebarPanel">
      <header className="sidebarPanelHeader">
        <div>
          <p className="eyebrow">Sidebar</p>
          <h3>Workspace metadata</h3>
        </div>
        <div className="sidebarTabStrip" role="tablist" aria-label="Sidebar tabs">
          <button
            type="button"
            className={`sidebarTab${activeTab === "workspace" ? " is-active" : ""}`}
            onClick={() => onChangeTab("workspace")}
          >
            Workspace
          </button>
          <button
            type="button"
            className={`sidebarTab${activeTab === "notifications" ? " is-active" : ""}`}
            onClick={() => onChangeTab("notifications")}
          >
            Notifications
          </button>
          <button
            type="button"
            className={`sidebarTab${activeTab === "ports" ? " is-active" : ""}`}
            onClick={() => onChangeTab("ports")}
          >
            Ports
          </button>
        </div>
      </header>

      <div className="sidebarPanelBody">
        {activeTab === "workspace" ? (
          workspace ? (
            <section className="sidebarCard">
              <h4>{workspace.custom_title || workspace.title}</h4>
              <dl className="sidebarDefinitionList">
                <div>
                  <dt>Directory</dt>
                  <dd>{workspace.current_directory}</dd>
                </div>
                <div>
                  <dt>Workspace ID</dt>
                  <dd>{workspace.id.slice(0, 12)}</dd>
                </div>
                <div>
                  <dt>Port ordinal</dt>
                  <dd>{workspace.port_ordinal}</dd>
                </div>
                <div>
                  <dt>Browser profile</dt>
                  <dd>{workspace.preferred_browser_profile_id ?? "Default"}</dd>
                </div>
              </dl>
            </section>
          ) : (
            <p className="sidebarEmpty">No workspace selected.</p>
          )
        ) : null}

        {activeTab === "notifications" ? (
          <section className="sidebarCard">
            <div className="sidebarCardHeader">
              <h4>Notifications</h4>
              <div className="sidebarCardActions">
                <button type="button" className="ghostButton" onClick={onPushNotification}>
                  Test
                </button>
                <button type="button" className="ghostButton" onClick={onClearNotifications}>
                  Clear
                </button>
              </div>
            </div>
            <div className="sidebarList">
              {(notifications?.items ?? []).map((entry) => (
                <article
                  key={entry.id}
                  className={`sidebarListItem level-${entry.level}${entry.is_read ? " is-read" : ""}`}
                >
                  <div className="sidebarListItemHeader">
                    <strong>{notificationLabel(entry)}</strong>
                    {!entry.is_read && onMarkNotificationRead ? (
                      <button
                        type="button"
                        className="ghostButton sidebarInlineButton"
                        onClick={() => void onMarkNotificationRead(entry.id)}
                      >
                        Mark read
                      </button>
                    ) : null}
                  </div>
                  <p>{entry.body}</p>
                </article>
              ))}
              {(notifications?.items ?? []).length === 0 ? (
                <p className="sidebarEmpty">No notifications yet.</p>
              ) : null}
            </div>
          </section>
        ) : null}

        {activeTab === "ports" ? (
          <section className="sidebarCard">
            <div className="sidebarCardHeader">
              <h4>Listening ports</h4>
              <button type="button" className="ghostButton" onClick={onRefreshPorts}>
                Refresh
              </button>
            </div>
            <div className="sidebarList">
              {(ports?.ports ?? []).map((entry) => (
                <article key={`${entry.protocol}-${entry.port}-${entry.address}`} className="sidebarListItem">
                  <strong>
                    {entry.protocol.toUpperCase()} {entry.port}
                  </strong>
                  <p>{entry.address}</p>
                  <p>
                    {entry.process_name || "Unknown process"}
                    {entry.pid ? ` • pid ${entry.pid}` : ""}
                  </p>
                </article>
              ))}
              {(ports?.ports ?? []).length === 0 ? (
                <p className="sidebarEmpty">No listening ports detected.</p>
              ) : null}
            </div>
          </section>
        ) : null}
      </div>
    </aside>
  );
}

export type NotificationLevel = "info" | "success" | "warning" | "error";

export interface NotificationEntry {
  id: string;
  title: string;
  body: string;
  level: NotificationLevel;
  created_at: number;
  is_read: boolean;
}

export interface NotificationAppSnapshot {
  revision: number;
  items: NotificationEntry[];
}

export interface ListeningPortSnapshot {
  port: number;
  protocol: string;
  address: string;
  process_name?: string | null;
  pid?: number | null;
}

export interface PortScanSnapshot {
  scanned_at: number;
  ports: ListeningPortSnapshot[];
}

//! Best-effort listening-port scanner used by the Phase 8 sidebar.
//!
//! The Swift app has platform-specific scanners. Here we keep a
//! pragmatic cross-platform implementation that prefers the standard
//! system tools available on the host and degrades gracefully to an
//! empty list when no command is available.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListeningPortSnapshot {
    pub port: u16,
    pub protocol: String,
    pub address: String,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortScanSnapshot {
    pub scanned_at: f64,
    pub ports: Vec<ListeningPortSnapshot>,
}

pub fn scan_listening_ports() -> PortScanSnapshot {
    let ports = scan_listening_ports_impl();
    PortScanSnapshot {
        scanned_at: unix_timestamp(),
        ports,
    }
}

fn scan_listening_ports_impl() -> Vec<ListeningPortSnapshot> {
    #[cfg(windows)]
    {
        scan_windows_netstat().unwrap_or_default()
    }

    #[cfg(unix)]
    {
        scan_unix_commands().unwrap_or_default()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn scan_windows_netstat() -> Option<Vec<ListeningPortSnapshot>> {
    let output = Command::new("netstat").args(["-ano"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_netstat_output(&String::from_utf8_lossy(&output.stdout), true))
}

#[cfg(unix)]
fn scan_unix_commands() -> Option<Vec<ListeningPortSnapshot>> {
    if let Some(output) = Command::new("ss").args(["-ltnp"]).output().ok() {
        if output.status.success() {
            return Some(parse_ss_output(&String::from_utf8_lossy(&output.stdout)));
        }
    }
    if let Some(output) = Command::new("netstat").args(["-an"]).output().ok() {
        if output.status.success() {
            return Some(parse_netstat_output(&String::from_utf8_lossy(&output.stdout), false));
        }
    }
    None
}

fn parse_netstat_output(output: &str, include_pid: bool) -> Vec<ListeningPortSnapshot> {
    let mut ports = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let columns: Vec<&str> = trimmed.split_whitespace().collect();
        if columns.is_empty() {
            continue;
        }

        let protocol = columns[0].to_ascii_lowercase();
        if include_pid {
            if columns.len() < 4 {
                continue;
            }
            let local_address = columns.get(1).copied().unwrap_or_default();
            let state = columns.get(3).copied().unwrap_or_default().to_ascii_uppercase();
            if state != "LISTENING" && state != "LISTEN" {
                continue;
            }
            let port = extract_port(local_address);
            let pid = columns.last().and_then(|value| value.parse::<u32>().ok());
            if let Some(port) = port {
                ports.push(ListeningPortSnapshot {
                    port,
                    protocol,
                    address: local_address.to_string(),
                    process_name: None,
                    pid,
                });
            }
        } else {
            if columns.len() < 2 {
                continue;
            }
            let local_address = columns[0];
            let state = columns.last().copied().unwrap_or_default().to_ascii_uppercase();
            if !state.contains("LISTEN") {
                continue;
            }
            if let Some(port) = extract_port(local_address) {
                ports.push(ListeningPortSnapshot {
                    port,
                    protocol,
                    address: local_address.to_string(),
                    process_name: None,
                    pid: None,
                });
            }
        }
    }
    ports.sort_by_key(|entry| entry.port);
    ports.dedup_by(|a, b| a.port == b.port && a.protocol == b.protocol && a.address == b.address);
    ports
}

#[cfg(unix)]
fn parse_ss_output(output: &str) -> Vec<ListeningPortSnapshot> {
    let mut ports = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("State ") {
            continue;
        }
        let columns: Vec<&str> = trimmed.split_whitespace().collect();
        if columns.len() < 4 {
            continue;
        }
        if !columns[0].eq_ignore_ascii_case("LISTEN") {
            continue;
        }
        let protocol = columns[0].to_ascii_lowercase();
        let local_address = columns[3];
        if let Some(port) = extract_port(local_address) {
            let process_name = trimmed.split("users:(").nth(1).and_then(|tail| {
                tail.split('"')
                    .nth(1)
                    .map(|value| value.to_string())
                    .or_else(|| Some(tail.trim().to_string()))
            });
            let pid = trimmed
                .split("pid=")
                .nth(1)
                .and_then(|tail| tail.split(',').next())
                .and_then(|value| value.parse::<u32>().ok());
            ports.push(ListeningPortSnapshot {
                port,
                protocol,
                address: local_address.to_string(),
                process_name,
                pid,
            });
        }
    }
    ports.sort_by_key(|entry| entry.port);
    ports.dedup_by(|a, b| a.port == b.port && a.protocol == b.protocol && a.address == b.address);
    ports
}

fn extract_port(endpoint: &str) -> Option<u16> {
    let candidate = endpoint.rsplit_once(':').map(|(_, port)| port).unwrap_or(endpoint);
    let digits = candidate.trim_matches(|ch| ch == ']' || ch == '[' || ch == '*');
    digits.parse::<u16>().ok()
}

fn unix_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

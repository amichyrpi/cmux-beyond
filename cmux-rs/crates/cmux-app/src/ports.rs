use cmux_core::ports::PortScanSnapshot;

#[tauri::command]
pub fn ports_scan() -> PortScanSnapshot {
    cmux_core::ports::scan_listening_ports()
}

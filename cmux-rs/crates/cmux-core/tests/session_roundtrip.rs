//! Round-trip test for the session snapshot schema. Ensures the Rust
//! serializer emits a structure that decodes back into the same
//! `AppSessionSnapshot` — a precondition for reading files written by
//! Swift and vice versa.

use std::fs;

use cmux_core::workspace::{
    AppSessionSnapshot, SessionPaneLayoutSnapshot, SessionPanelSnapshot, SessionPanelType,
    SessionPersistenceStore, SessionSidebarSelection, SessionSidebarSnapshot,
    SessionSnapshotSchema, SessionSplitLayoutSnapshot, SessionSplitOrientation,
    SessionTabManagerSnapshot, SessionTerminalPanelSnapshot, SessionWindowSnapshot,
    SessionWorkspaceLayoutSnapshot, SessionWorkspaceSnapshot,
};
use uuid::Uuid;

fn sample_snapshot() -> AppSessionSnapshot {
    let panel_id = Uuid::new_v4();
    let other_panel_id = Uuid::new_v4();
    AppSessionSnapshot {
        version: SessionSnapshotSchema::CURRENT_VERSION,
        created_at: 0.0,
        windows: vec![SessionWindowSnapshot {
            frame: None,
            display: None,
            tab_manager: SessionTabManagerSnapshot {
                selected_workspace_index: Some(0),
                workspaces: vec![SessionWorkspaceSnapshot {
                    process_title: "ws".into(),
                    custom_title: None,
                    custom_description: None,
                    custom_color: None,
                    is_pinned: false,
                    current_directory: "/tmp".into(),
                    focused_panel_id: Some(panel_id),
                    layout: SessionWorkspaceLayoutSnapshot::Split {
                        split: SessionSplitLayoutSnapshot {
                            orientation: SessionSplitOrientation::Horizontal,
                            divider_position: 0.42,
                            first: Box::new(SessionWorkspaceLayoutSnapshot::Pane {
                                pane: SessionPaneLayoutSnapshot {
                                    panel_ids: vec![panel_id],
                                    selected_panel_id: Some(panel_id),
                                },
                            }),
                            second: Box::new(SessionWorkspaceLayoutSnapshot::Pane {
                                pane: SessionPaneLayoutSnapshot {
                                    panel_ids: vec![other_panel_id],
                                    selected_panel_id: Some(other_panel_id),
                                },
                            }),
                        },
                    },
                    panels: vec![
                        SessionPanelSnapshot {
                            id: panel_id,
                            panel_type: SessionPanelType::Terminal,
                            title: Some("shell".into()),
                            custom_title: None,
                            directory: Some("/tmp".into()),
                            is_pinned: false,
                            is_manually_unread: false,
                            git_branch: None,
                            listening_ports: vec![],
                            tty_name: Some("ttys000".into()),
                            terminal: Some(SessionTerminalPanelSnapshot {
                                working_directory: Some("/tmp".into()),
                                scrollback: None,
                            }),
                            browser: None,
                            markdown: None,
                        },
                        SessionPanelSnapshot {
                            id: other_panel_id,
                            panel_type: SessionPanelType::Terminal,
                            title: Some("shell2".into()),
                            custom_title: None,
                            directory: Some("/tmp".into()),
                            is_pinned: false,
                            is_manually_unread: false,
                            git_branch: None,
                            listening_ports: vec![],
                            tty_name: None,
                            terminal: Some(SessionTerminalPanelSnapshot::default()),
                            browser: None,
                            markdown: None,
                        },
                    ],
                    status_entries: vec![],
                    log_entries: vec![],
                    progress: None,
                    git_branch: None,
                }],
            },
            sidebar: SessionSidebarSnapshot {
                is_visible: true,
                selection: SessionSidebarSelection::Tabs,
                width: Some(200.0),
            },
        }],
    }
}

#[test]
fn snapshot_round_trips_through_json() {
    let snapshot = sample_snapshot();
    let json = serde_json::to_string(&snapshot).unwrap();
    let decoded: AppSessionSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, snapshot);
}

#[test]
fn store_load_returns_none_for_missing_file() {
    let tmp = tempdir_sibling("missing");
    let missing = tmp.join("does-not-exist.json");
    assert!(SessionPersistenceStore::load(&missing).unwrap().is_none());
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn store_save_then_load_round_trips() {
    let tmp = tempdir_sibling("save");
    let file = tmp.join("session.json");
    let snapshot = sample_snapshot();
    SessionPersistenceStore::save(&file, &snapshot).unwrap();
    let loaded = SessionPersistenceStore::load(&file).unwrap().unwrap();
    assert_eq!(loaded, snapshot);
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn store_load_rejects_version_mismatch() {
    let tmp = tempdir_sibling("version");
    let file = tmp.join("session.json");
    let mut snapshot = sample_snapshot();
    snapshot.version = 99;
    fs::write(&file, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert!(SessionPersistenceStore::load(&file).unwrap().is_none());
    fs::remove_dir_all(&tmp).ok();
}

fn tempdir_sibling(label: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "cmux-core-session-test-{}-{}",
        label,
        std::process::id()
    ));
    fs::create_dir_all(&base).unwrap();
    base
}

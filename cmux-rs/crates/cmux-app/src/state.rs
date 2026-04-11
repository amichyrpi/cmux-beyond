use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use cmux_core::bonsplit::{PaneId, TabId};
use cmux_core::workspace::{TabManager, TabManagerSnapshot, Workspace};

const STATE_EVENT: &str = "cmux:state";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub revision: u64,
    pub tab_manager: TabManagerSnapshot,
}

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<Mutex<AppStateInner>>,
}

#[derive(Debug)]
pub(crate) struct AppStateInner {
    revision: u64,
    pub(crate) tab_manager: TabManager,
}

impl Default for AppStateInner {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut tab_manager = TabManager::new();
        let workspace = Workspace::with_starting_tab(
            "Workspace 1",
            cwd.to_string_lossy().to_string(),
            "Welcome",
        );
        tab_manager.push_workspace(workspace);
        Self {
            revision: 0,
            tab_manager,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AppStateInner::default())),
        }
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let inner = self.inner.lock();
        AppSnapshot {
            revision: inner.revision,
            tab_manager: inner.tab_manager.snapshot(),
        }
    }

    pub fn emit(&self, app: &AppHandle) -> tauri::Result<AppSnapshot> {
        let snapshot = self.snapshot();
        app.emit(STATE_EVENT, snapshot.clone())?;
        Ok(snapshot)
    }

    pub(crate) fn mutate<F>(&self, app: &AppHandle, f: F) -> tauri::Result<AppSnapshot>
    where
        F: FnOnce(&mut AppStateInner) -> bool,
    {
        let changed = {
            let mut inner = self.inner.lock();
            let changed = f(&mut inner);
            if changed {
                inner.revision = inner.revision.saturating_add(1);
            }
            changed
        };

        let snapshot = self.snapshot();
        if changed {
            app.emit(STATE_EVENT, snapshot.clone())?;
        }
        Ok(snapshot)
    }
}

fn parse_uuid(raw: &str) -> Result<Uuid, String> {
    Uuid::parse_str(raw).map_err(|err| format!("invalid uuid '{raw}': {err}"))
}

fn parse_pane_id(raw: &str) -> Result<PaneId, String> {
    parse_uuid(raw).map(PaneId::from_uuid)
}

fn parse_tab_id(raw: &str) -> Result<TabId, String> {
    parse_uuid(raw).map(TabId::from_uuid)
}

pub fn app_state() -> AppState {
    AppState::new()
}

pub fn state_event_name() -> &'static str {
    STATE_EVENT
}

pub fn mutate_workspace_by_id<F>(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    workspace_id: Uuid,
    f: F,
) -> Result<AppSnapshot, String>
where
    F: FnOnce(&mut Workspace) -> bool,
{
    state
        .mutate(&app, |inner| {
            let Some(workspace) = inner.tab_manager.workspace_mut(workspace_id) else {
                return false;
            };
            f(workspace)
        })
        .map_err(|err| err.to_string())
}

pub fn workspace_id_from(raw: &str) -> Result<Uuid, String> {
    parse_uuid(raw)
}

pub fn pane_id_from(raw: &str) -> Result<PaneId, String> {
    parse_pane_id(raw)
}

pub fn tab_id_from(raw: &str) -> Result<TabId, String> {
    parse_tab_id(raw)
}

use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use cmux_core::terminal::{TerminalSearchResult, TerminalSession, TerminalSize, TerminalSnapshot};

const TERMINAL_STATE_EVENT: &str = "cmux:terminal";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalAppSnapshot {
    pub revision: u64,
    pub sessions: Vec<TerminalSnapshot>,
}

#[derive(Clone)]
pub struct TerminalState {
    inner: Arc<Mutex<TerminalStateInner>>,
}

struct TerminalStateInner {
    revision: u64,
    sessions: BTreeMap<String, TerminalSession>,
}

impl Default for TerminalStateInner {
    fn default() -> Self {
        Self {
            revision: 0,
            sessions: BTreeMap::new(),
        }
    }
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TerminalStateInner::default())),
        }
    }

    pub fn snapshot(&self) -> TerminalAppSnapshot {
        let mut inner = self.inner.lock();
        let sessions = inner
            .sessions
            .values_mut()
            .filter_map(|session| session.snapshot().ok())
            .collect();
        TerminalAppSnapshot {
            revision: inner.revision,
            sessions,
        }
    }

    pub fn emit(&self, app: &AppHandle) -> tauri::Result<TerminalAppSnapshot> {
        let snapshot = self.snapshot();
        app.emit(TERMINAL_STATE_EVENT, snapshot.clone())?;
        Ok(snapshot)
    }

    fn mutate<F>(&self, app: &AppHandle, f: F) -> tauri::Result<TerminalAppSnapshot>
    where
        F: FnOnce(&mut TerminalStateInner) -> bool,
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
            app.emit(TERMINAL_STATE_EVENT, snapshot.clone())?;
        }
        Ok(snapshot)
    }
}

fn get_or_create_session<'a>(
    inner: &'a mut TerminalStateInner,
    session_key: &str,
    working_directory: Option<String>,
) -> Result<&'a mut TerminalSession, String> {
    if !inner.sessions.contains_key(session_key) {
        let cwd = working_directory.map(std::path::PathBuf::from);
        let session = TerminalSession::spawn_shell(cwd, TerminalSize::default())
            .map_err(|err| err.to_string())?;
        inner.sessions.insert(session_key.to_string(), session);
    }
    inner
        .sessions
        .get_mut(session_key)
        .ok_or_else(|| format!("terminal session '{session_key}' not found"))
}

#[tauri::command]
pub fn terminal_state(state: State<'_, TerminalState>) -> TerminalAppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn terminal_ensure(
    app: AppHandle,
    state: State<'_, TerminalState>,
    session_key: String,
    working_directory: Option<String>,
) -> Result<TerminalSnapshot, String> {
    state
        .mutate(&app, |inner| {
            get_or_create_session(inner, &session_key, working_directory.clone()).is_ok()
        })
        .map_err(|err| err.to_string())?;
    terminal_snapshot(state, session_key)
}

#[tauri::command]
pub fn terminal_snapshot(
    state: State<'_, TerminalState>,
    session_key: String,
) -> Result<TerminalSnapshot, String> {
    let mut inner = state.inner.lock();
    let session = inner
        .sessions
        .get_mut(&session_key)
        .ok_or_else(|| format!("terminal session '{session_key}' not found"))?;
    session.snapshot().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn terminal_input(
    app: AppHandle,
    state: State<'_, TerminalState>,
    session_key: String,
    input: String,
) -> Result<TerminalSnapshot, String> {
    state
        .mutate(&app, |inner| {
            let Some(session) = inner.sessions.get_mut(&session_key) else {
                return false;
            };
            session.write_input(input.as_bytes()).is_ok()
        })
        .map_err(|err| err.to_string())?;
    terminal_snapshot(state, session_key)
}

#[tauri::command]
pub fn terminal_resize(
    app: AppHandle,
    state: State<'_, TerminalState>,
    session_key: String,
    columns: usize,
    rows: usize,
) -> Result<TerminalSnapshot, String> {
    state
        .mutate(&app, |inner| {
            let Some(session) = inner.sessions.get_mut(&session_key) else {
                return false;
            };
            session.resize(TerminalSize::new(columns, rows)).is_ok()
        })
        .map_err(|err| err.to_string())?;
    terminal_snapshot(state, session_key)
}

#[tauri::command]
pub fn terminal_search(
    state: State<'_, TerminalState>,
    session_key: String,
    query: String,
) -> Result<TerminalSearchResult, String> {
    let mut inner = state.inner.lock();
    let session = inner
        .sessions
        .get_mut(&session_key)
        .ok_or_else(|| format!("terminal session '{session_key}' not found"))?;
    session
        .search_visible_text_matches(&query)
        .map_err(|err| err.to_string())
}

pub fn terminal_state_handle() -> TerminalState {
    TerminalState::new()
}

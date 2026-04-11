use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use cmux_core::browser::{BrowserPopupRequest, BrowserSession, BrowserSessionSnapshot};

const BROWSER_STATE_EVENT: &str = "cmux:browser";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAppSnapshot {
    pub revision: u64,
    pub sessions: Vec<BrowserSessionSnapshot>,
}

#[derive(Debug, Clone)]
pub struct BrowserState {
    inner: Arc<Mutex<BrowserStateInner>>,
}

#[derive(Debug)]
struct BrowserStateInner {
    revision: u64,
    sessions: BTreeMap<Uuid, BrowserSession>,
    label_to_session_id: BTreeMap<String, Uuid>,
}

impl Default for BrowserStateInner {
    fn default() -> Self {
        Self {
            revision: 0,
            sessions: BTreeMap::new(),
            label_to_session_id: BTreeMap::new(),
        }
    }
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BrowserStateInner::default())),
        }
    }

    pub fn snapshot(&self) -> BrowserAppSnapshot {
        let inner = self.inner.lock();
        BrowserAppSnapshot {
            revision: inner.revision,
            sessions: inner.sessions.values().map(BrowserSession::snapshot).collect(),
        }
    }

    pub fn emit(&self, app: &AppHandle) -> tauri::Result<BrowserAppSnapshot> {
        let snapshot = self.snapshot();
        app.emit(BROWSER_STATE_EVENT, snapshot.clone())?;
        Ok(snapshot)
    }

    fn mutate<F>(&self, app: &AppHandle, f: F) -> tauri::Result<BrowserAppSnapshot>
    where
        F: FnOnce(&mut BrowserStateInner) -> bool,
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
            app.emit(BROWSER_STATE_EVENT, snapshot.clone())?;
        }
        Ok(snapshot)
    }
}

fn session_by_id_mut<'a>(
    inner: &'a mut BrowserStateInner,
    session_id: Uuid,
) -> Option<&'a mut BrowserSession> {
    inner.sessions.get_mut(&session_id)
}

fn session_by_label_mut<'a>(
    inner: &'a mut BrowserStateInner,
    window_label: &str,
) -> Option<&'a mut BrowserSession> {
    let session_id = inner.label_to_session_id.get(window_label).copied()?;
    inner.sessions.get_mut(&session_id)
}

fn parse_uuid(raw: &str) -> Result<Uuid, String> {
    Uuid::parse_str(raw).map_err(|err| format!("invalid uuid '{raw}': {err}"))
}

fn parse_url(raw: &str) -> Result<Url, String> {
    Url::parse(raw).map_err(|err| format!("invalid URL '{raw}': {err}"))
}

fn browser_title(session: &BrowserSession) -> String {
    session
        .title
        .clone()
        .or_else(|| session.current_url.clone())
        .unwrap_or_else(|| "Browser".to_string())
}

fn open_or_update_window(app: &AppHandle, session: &BrowserSession) -> Result<(), String> {
    let url = session
        .current_url
        .as_deref()
        .unwrap_or("about:blank");
    let parsed = parse_url(url)?;
    let title = browser_title(session);

    if let Some(window) = app.get_webview_window(&session.window_label) {
        window.navigate(parsed).map_err(|err| err.to_string())?;
        window.set_title(&title).map_err(|err| err.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        session.window_label.clone(),
        WebviewUrl::External(parsed),
    )
    .title(title)
    .build()
    .map_err(|err| err.to_string())?;

    window
        .set_title(&browser_title(session))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn ensure_session_by_label(
    inner: &mut BrowserStateInner,
    window_label: String,
    url: Option<String>,
    title: Option<String>,
) -> &mut BrowserSession {
    let session_id = if let Some(session_id) = inner.label_to_session_id.get(&window_label).copied() {
        session_id
    } else {
        let mut session = BrowserSession::with_window_label(None, None, url, window_label.clone());
        if let Some(title) = title.clone() {
            session.set_title(Some(title));
        }
        let session_id = session.id;
        inner.label_to_session_id.insert(window_label.clone(), session_id);
        inner.sessions.insert(session_id, session);
        session_id
    };
    inner.sessions.get_mut(&session_id).expect("session inserted")
}

#[tauri::command]
pub fn browser_state(state: State<'_, BrowserState>) -> BrowserAppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn browser_ensure(
    app: AppHandle,
    state: State<'_, BrowserState>,
    window_label: String,
    url: Option<String>,
    title: Option<String>,
) -> Result<BrowserSessionSnapshot, String> {
    state
        .mutate(&app, |inner| {
            let session = ensure_session_by_label(inner, window_label.clone(), url.clone(), title.clone());
            if let Some(next_url) = url.clone() {
                session.navigate(next_url);
            }
            if title.is_some() {
                session.set_title(title.clone());
            }
            true
        })
        .map_err(|err| err.to_string())?;
    browser_snapshot_by_label(state, window_label)
}

#[tauri::command]
pub fn browser_snapshot_by_label(
    state: State<'_, BrowserState>,
    window_label: String,
) -> Result<BrowserSessionSnapshot, String> {
    let mut inner = state.inner.lock();
    let session = session_by_label_mut(&mut inner, &window_label)
        .ok_or_else(|| format!("browser session '{window_label}' not found"))?;
    Ok(session.snapshot())
}

#[tauri::command]
pub fn browser_open(
    app: AppHandle,
    state: State<'_, BrowserState>,
    request: BrowserPopupRequest,
) -> Result<BrowserAppSnapshot, String> {
    state
        .mutate(&app, |inner| {
            let mut session = BrowserSession::new(None, None, Some(request.url.clone()));
            if let Some(title) = request.target_title {
                session.set_title(Some(title));
            }
            let session_id = session.id;
            let window_label = session.window_label.clone();
            open_or_update_window(&app, &session).ok();
            inner.sessions.insert(session_id, session);
            inner.label_to_session_id.insert(window_label, session_id);
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_navigate(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
    url: String,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    let parsed = parse_url(&url)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = session_by_id_mut(inner, session_id) else {
                return false;
            };
            session.navigate(url.clone());
            if let Some(window) = app.get_webview_window(&session.window_label) {
                window.navigate(parsed.clone()).ok();
            } else {
                open_or_update_window(&app, session).ok();
            }
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_reload(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = session_by_id_mut(inner, session_id) else {
                return false;
            };
            let Some(url) = session.reload() else {
                return false;
            };
            let parsed = parse_url(&url).ok();
            if let Some(window) = app.get_webview_window(&session.window_label) {
                if let Some(parsed) = parsed {
                    window.navigate(parsed).ok();
                }
            } else {
                open_or_update_window(&app, session).ok();
            }
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_back(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = session_by_id_mut(inner, session_id) else {
                return false;
            };
            let Some(url) = session.go_back() else {
                return false;
            };
            if let Ok(parsed) = parse_url(&url) {
                if let Some(window) = app.get_webview_window(&session.window_label) {
                    window.navigate(parsed).ok();
                }
            }
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_forward(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = session_by_id_mut(inner, session_id) else {
                return false;
            };
            let Some(url) = session.go_forward() else {
                return false;
            };
            if let Ok(parsed) = parse_url(&url) {
                if let Some(window) = app.get_webview_window(&session.window_label) {
                    window.navigate(parsed).ok();
                }
            }
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_set_title(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
    title: Option<String>,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = session_by_id_mut(inner, session_id) else {
                return false;
            };
            session.set_title(title.clone());
            if let Some(window) = app.get_webview_window(&session.window_label) {
                window
                    .set_title(&browser_title(session))
                    .map_err(|err| err.to_string())
                    .ok();
            }
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn browser_close(
    app: AppHandle,
    state: State<'_, BrowserState>,
    session_id: String,
) -> Result<BrowserAppSnapshot, String> {
    let session_id = parse_uuid(&session_id)?;
    state
        .mutate(&app, |inner| {
            let Some(session) = inner.sessions.remove(&session_id) else {
                return false;
            };
            if let Some(window) = app.get_webview_window(&session.window_label) {
                window.close().ok();
            }
            true
        })
        .map_err(|err| err.to_string())
}

pub fn browser_state_handle() -> BrowserState {
    BrowserState::new()
}

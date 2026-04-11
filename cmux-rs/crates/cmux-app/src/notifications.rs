use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

use cmux_core::notifications::{
    notification_timestamp, NotificationEntry, NotificationLevel,
};

const NOTIFICATION_STATE_EVENT: &str = "cmux:notifications";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAppSnapshot {
    pub revision: u64,
    pub items: Vec<NotificationEntry>,
}

#[derive(Debug, Clone)]
pub struct NotificationState {
    inner: Arc<Mutex<NotificationStateInner>>,
}

#[derive(Debug)]
struct NotificationStateInner {
    revision: u64,
    items: Vec<NotificationEntry>,
}

impl Default for NotificationStateInner {
    fn default() -> Self {
        Self {
            revision: 0,
            items: Vec::new(),
        }
    }
}

impl NotificationState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(NotificationStateInner::default())),
        }
    }

    pub fn snapshot(&self) -> NotificationAppSnapshot {
        let inner = self.inner.lock();
        NotificationAppSnapshot {
            revision: inner.revision,
            items: inner.items.clone(),
        }
    }

    pub fn emit(&self, app: &AppHandle) -> tauri::Result<NotificationAppSnapshot> {
        let snapshot = self.snapshot();
        app.emit(NOTIFICATION_STATE_EVENT, snapshot.clone())?;
        Ok(snapshot)
    }

    fn mutate<F>(&self, app: &AppHandle, f: F) -> tauri::Result<NotificationAppSnapshot>
    where
        F: FnOnce(&mut NotificationStateInner) -> bool,
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
            app.emit(NOTIFICATION_STATE_EVENT, snapshot.clone())?;
        }
        Ok(snapshot)
    }
}

#[tauri::command]
pub fn notifications_state(state: State<'_, NotificationState>) -> NotificationAppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn notifications_push(
    app: AppHandle,
    state: State<'_, NotificationState>,
    title: String,
    body: String,
    level: Option<NotificationLevel>,
) -> Result<NotificationAppSnapshot, String> {
    let level = level.unwrap_or(NotificationLevel::Info);
    state
        .mutate(&app, |inner| {
            inner.items.insert(
                0,
                NotificationEntry {
                    id: Uuid::new_v4(),
                    title: title.clone(),
                    body: body.clone(),
                    level,
                    created_at: notification_timestamp(),
                    is_read: false,
                },
            );
            true
        })
        .map_err(|err| err.to_string())?;

    let severity = match level {
        NotificationLevel::Info => "Info",
        NotificationLevel::Success => "Success",
        NotificationLevel::Warning => "Warning",
        NotificationLevel::Error => "Error",
    };
    let _ = app
        .notification()
        .builder()
        .title(format!("cmux {severity}"))
        .body(body)
        .show();

    Ok(state.snapshot())
}

#[tauri::command]
pub fn notifications_mark_read(
    app: AppHandle,
    state: State<'_, NotificationState>,
    notification_id: String,
) -> Result<NotificationAppSnapshot, String> {
    let id = Uuid::parse_str(&notification_id)
        .map_err(|err| format!("invalid notification id '{notification_id}': {err}"))?;
    state
        .mutate(&app, |inner| {
            if let Some(item) = inner.items.iter_mut().find(|item| item.id == id) {
                item.is_read = true;
                true
            } else {
                false
            }
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn notifications_clear(
    app: AppHandle,
    state: State<'_, NotificationState>,
) -> Result<NotificationAppSnapshot, String> {
    state
        .mutate(&app, |inner| {
            if inner.items.is_empty() {
                false
            } else {
                inner.items.clear();
                true
            }
        })
        .map_err(|err| err.to_string())
}

pub fn notification_state_handle() -> NotificationState {
    NotificationState::new()
}

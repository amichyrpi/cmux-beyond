//! Browser panel session model for the Rust rewrite.
//!
//! Phase 7 begins here: a lightweight browser-session data model that
//! can back Tauri webview windows while we port the richer Swift
//! browser panel behavior in smaller slices.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserHistoryEntry {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserPopupRequest {
    pub url: String,
    #[serde(default)]
    pub opener_window_label: Option<String>,
    #[serde(default)]
    pub target_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserSessionSnapshot {
    pub id: Uuid,
    pub window_label: String,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    #[serde(default)]
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub current_url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub page_zoom: f64,
    pub developer_tools_visible: bool,
    pub should_render_webview: bool,
    #[serde(default)]
    pub back_history: Vec<BrowserHistoryEntry>,
    #[serde(default)]
    pub forward_history: Vec<BrowserHistoryEntry>,
}

#[derive(Debug, Clone)]
pub struct BrowserSession {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub window_label: String,
    pub profile_id: Option<Uuid>,
    pub current_url: Option<String>,
    pub title: Option<String>,
    pub page_zoom: f64,
    pub developer_tools_visible: bool,
    pub should_render_webview: bool,
    pub back_history: Vec<BrowserHistoryEntry>,
    pub forward_history: Vec<BrowserHistoryEntry>,
}

impl BrowserSession {
    pub fn new(
        workspace_id: Option<Uuid>,
        profile_id: Option<Uuid>,
        current_url: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let window_label = format!("browser-{}", id.simple());
        Self::with_window_label(workspace_id, profile_id, current_url, window_label)
    }

    pub fn with_window_label(
        workspace_id: Option<Uuid>,
        profile_id: Option<Uuid>,
        current_url: Option<String>,
        window_label: String,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            workspace_id,
            window_label,
            profile_id,
            current_url,
            title: None,
            page_zoom: 1.0,
            developer_tools_visible: false,
            should_render_webview: true,
            back_history: Vec::new(),
            forward_history: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> BrowserSessionSnapshot {
        BrowserSessionSnapshot {
            id: self.id,
            window_label: self.window_label.clone(),
            workspace_id: self.workspace_id,
            profile_id: self.profile_id,
            current_url: self.current_url.clone(),
            title: self.title.clone(),
            page_zoom: self.page_zoom,
            developer_tools_visible: self.developer_tools_visible,
            should_render_webview: self.should_render_webview,
            back_history: self.back_history.clone(),
            forward_history: self.forward_history.clone(),
        }
    }

    pub fn navigate(&mut self, url: impl Into<String>) {
        let next = url.into();
        if let Some(current) = self.current_url.take() {
            if current != next {
                self.back_history.push(BrowserHistoryEntry {
                    url: current,
                    title: self.title.clone(),
                });
                self.forward_history.clear();
            }
        }
        self.current_url = Some(next);
    }

    pub fn go_back(&mut self) -> Option<String> {
        let entry = self.back_history.pop()?;
        if let Some(current) = self.current_url.take() {
            self.forward_history.push(BrowserHistoryEntry {
                url: current,
                title: self.title.clone(),
            });
        }
        self.current_url = Some(entry.url.clone());
        self.title = entry.title;
        Some(entry.url)
    }

    pub fn go_forward(&mut self) -> Option<String> {
        let entry = self.forward_history.pop()?;
        if let Some(current) = self.current_url.take() {
            self.back_history.push(BrowserHistoryEntry {
                url: current,
                title: self.title.clone(),
            });
        }
        self.current_url = Some(entry.url.clone());
        self.title = entry.title;
        Some(entry.url)
    }

    pub fn reload(&self) -> Option<String> {
        self.current_url.clone()
    }

    pub fn set_title(&mut self, title: impl Into<Option<String>>) {
        self.title = title.into();
    }

    pub fn set_page_zoom(&mut self, page_zoom: f64) {
        self.page_zoom = page_zoom.clamp(0.25, 3.0);
    }

    pub fn set_developer_tools_visible(&mut self, visible: bool) {
        self.developer_tools_visible = visible;
    }
}

pub fn browser_window_label(session_id: Uuid) -> String {
    format!("browser-{}", session_id.simple())
}

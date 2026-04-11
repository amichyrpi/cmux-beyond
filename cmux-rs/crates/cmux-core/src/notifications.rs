//! Lightweight notification sidebar model for the Phase 8 rewrite.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub level: NotificationLevel,
    pub created_at: f64,
    pub is_read: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSnapshot {
    pub revision: u64,
    pub items: Vec<NotificationEntry>,
}

pub fn notification_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

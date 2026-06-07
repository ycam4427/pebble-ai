//! Persisted records surfaced to the UI (Trash, Action History, saved locations).

use serde::{Deserialize, Serialize};

/// A file or folder currently in the AI Trash (recoverable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashItem {
    pub id: String,
    pub original_path: String,
    pub trash_path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub deleted_at: String,
    pub expires_at: String,
    pub restored_at: Option<String>,
}

/// One executed (or failed/undone) operation in the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    pub id: String,
    pub plan_id: Option<String>,
    pub op_index: i64,
    pub kind: String,
    pub tier: u8,
    pub source: String,
    pub destination: Option<String>,
    /// "executed" | "failed" | "undone"
    pub status: String,
    pub error: Option<String>,
    pub executed_at: String,
    pub undone_at: Option<String>,
}

/// A frequently-used / remembered location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub path: String,
    pub label: Option<String>,
    pub kind: Option<String>,
    pub use_count: i64,
    pub last_used: Option<String>,
}

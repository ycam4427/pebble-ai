//! Conversation/message persistence types and generation statistics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    /// "user" | "assistant" | "system"
    pub role: String,
    pub content: String,
    pub created_at: String,
    /// JSON snapshot of any actions proposed in this turn (for the transcript).
    #[serde(default)]
    pub actions_json: Option<String>,
}

/// Generation performance, surfaced in the UI (tokens/sec etc.).
#[derive(Debug, Clone, Serialize)]
pub struct GenStats {
    pub model: String,
    pub tokens: u64,
    pub tokens_per_sec: f64,
    pub total_ms: u64,
}

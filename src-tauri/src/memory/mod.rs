//! Local memory: conversation context for the model, plus remembered locations.
//! Everything is stored on-device via the `db` layer — no cloud, ever.

use crate::ai::ChatMessage;
use crate::db;
use crate::models::Location;
use anyhow::Result;
use rusqlite::Connection;

/// Build the message list sent to Ollama: the system prompt followed by the most
/// recent conversation turns (capped to keep the context window small).
pub fn conversation_messages(
    conn: &Connection,
    conversation_id: &str,
    system: String,
    max_turns: usize,
) -> Result<Vec<ChatMessage>> {
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: system,
    }];

    let history = db::list_messages(conn, conversation_id)?;
    let start = history.len().saturating_sub(max_turns * 2);
    for m in history.into_iter().skip(start) {
        if m.role == "user" || m.role == "assistant" {
            messages.push(ChatMessage {
                role: m.role,
                content: m.content,
            });
        }
    }
    Ok(messages)
}

/// Record that a location was used (frequency + recency), for future suggestions.
pub fn remember_location(conn: &Connection, path: &str, label: Option<&str>) {
    let _ = db::record_location(conn, path, label, None);
}

/// Most frequently/recently used locations.
pub fn frequent_locations(conn: &Connection, n: i64) -> Vec<Location> {
    db::list_locations(conn, n).unwrap_or_default()
}

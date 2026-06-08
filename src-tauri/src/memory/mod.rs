//! Local memory: conversation context for the model, plus remembered locations.
//! Everything is stored on-device via the `db` layer — no cloud, ever.

use crate::ai::ChatMessage;
use crate::db;
use crate::models::Location;
use anyhow::Result;
use chrono::NaiveDate;
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

/// A compact "what Pebble remembers about you" block for the prompt. Empty when
/// there's nothing stored. Dated events are annotated relative to `today` (so he
/// can naturally say "that's in 3 days"). `today` is YYYY-MM-DD.
pub fn recall_block(conn: &Connection, name: &str, today: &str) -> String {
    let items = db::list_memory(conn).unwrap_or_default();
    if items.is_empty() {
        return String::new();
    }
    let who = if name.trim().is_empty() {
        "them".to_string()
    } else {
        name.trim().to_string()
    };
    let today_date = NaiveDate::parse_from_str(today, "%Y-%m-%d").ok();
    let mut facts: Vec<String> = Vec::new();
    let mut events: Vec<String> = Vec::new();
    for m in &items {
        match &m.event_date {
            Some(d) if !d.is_empty() => {
                let when = match (today_date, NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) {
                    (Some(t), Some(ev)) => {
                        let days = (ev - t).num_days();
                        if days == 0 {
                            " — that's TODAY".to_string()
                        } else if days > 0 {
                            format!(" — in {days} day(s) (on {d})")
                        } else {
                            format!(" — {} day(s) ago (on {d})", -days)
                        }
                    }
                    _ => format!(" (on {d})"),
                };
                events.push(format!("{}{}", m.content, when));
            }
            _ => facts.push(m.content.clone()),
        }
    }
    let mut out = format!(
        "\nWHAT YOU REMEMBER ABOUT {who} (weave these in naturally when relevant — don't dump them all at once, and don't recite the list):\n"
    );
    for e in events.iter().take(10) {
        out.push_str(&format!("  - {e}\n"));
    }
    for f in facts.iter().take(12) {
        out.push_str(&format!("  - {f}\n"));
    }
    out
}

//! Local SQLite data-access layer. Plain functions over a `&Connection`; the
//! connection itself lives behind a `Mutex` in the app state.

use crate::models::{ActionLogEntry, Conversation, Location, Message, TrashItem};
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;
use uuid::Uuid;

/// A logged operation enriched with its reversal data (internal to undo).
#[derive(Debug, Clone)]
#[allow(dead_code)] // source/destination retained for diagnostics
pub struct LoggedOp {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub destination: Option<String>,
    pub status: String,
    pub undo_data: Option<String>,
}

/// Open (creating if needed) the database and apply the schema.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(include_str!("schema.sql"))?;
    Ok(conn)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------- conversations

pub fn create_conversation(conn: &Connection, title: &str) -> Result<Conversation> {
    let c = Conversation {
        id: Uuid::new_v4().to_string(),
        title: title.to_string(),
        created_at: now(),
        updated_at: now(),
    };
    conn.execute(
        "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![c.id, c.title, c.created_at, c.updated_at],
    )?;
    Ok(c)
}

fn row_to_conversation(r: &Row) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: r.get(0)?,
        title: r.get(1)?,
        created_at: r.get(2)?,
        updated_at: r.get(3)?,
    })
}

pub fn list_conversations(conn: &Connection) -> Result<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_conversation)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[allow(dead_code)]
pub fn get_conversation(conn: &Connection, id: &str) -> Result<Option<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?1",
    )?;
    Ok(stmt
        .query_row(params![id], row_to_conversation)
        .optional()?)
}

pub fn touch_conversation(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET updated_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;
    Ok(())
}

pub fn rename_conversation(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, title, now()],
    )?;
    Ok(())
}

pub fn delete_conversation(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
    Ok(())
}

// --------------------------------------------------------------------- messages

pub fn insert_message(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
    actions_json: Option<&str>,
) -> Result<Message> {
    let m = Message {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        created_at: now(),
        actions_json: actions_json.map(|s| s.to_string()),
    };
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, actions_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![m.id, m.conversation_id, m.role, m.content, m.actions_json, m.created_at],
    )?;
    touch_conversation(conn, conversation_id)?;
    Ok(m)
}

pub fn list_messages(conn: &Connection, conversation_id: &str) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, actions_json, created_at
         FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |r| {
        Ok(Message {
            id: r.get(0)?,
            conversation_id: r.get(1)?,
            role: r.get(2)?,
            content: r.get(3)?,
            actions_json: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ------------------------------------------------------------------- action_log

#[allow(clippy::too_many_arguments)]
pub fn insert_action_log(
    conn: &Connection,
    plan_id: Option<&str>,
    op_index: i64,
    kind: &str,
    tier: u8,
    source: &str,
    destination: Option<&str>,
    status: &str,
    undo_data_json: Option<&str>,
    error: Option<&str>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO action_log
           (id, plan_id, op_index, kind, tier, source, destination, status, undo_data_json, error, executed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id, plan_id, op_index, kind, tier, source, destination, status, undo_data_json, error, now()
        ],
    )?;
    Ok(id)
}

fn row_to_log(r: &Row) -> rusqlite::Result<ActionLogEntry> {
    Ok(ActionLogEntry {
        id: r.get(0)?,
        plan_id: r.get(1)?,
        op_index: r.get(2)?,
        kind: r.get(3)?,
        tier: r.get::<_, i64>(4)? as u8,
        source: r.get(5)?,
        destination: r.get(6)?,
        status: r.get(7)?,
        error: r.get(8)?,
        executed_at: r.get(9)?,
        undone_at: r.get(10)?,
    })
}

pub fn list_action_log(conn: &Connection, limit: i64) -> Result<Vec<ActionLogEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, plan_id, op_index, kind, tier, source, destination, status, error, executed_at, undone_at
         FROM action_log ORDER BY executed_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], row_to_log)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn row_to_logged_op(r: &Row) -> rusqlite::Result<LoggedOp> {
    Ok(LoggedOp {
        id: r.get(0)?,
        kind: r.get(1)?,
        source: r.get(2)?,
        destination: r.get(3)?,
        status: r.get(4)?,
        undo_data: r.get(5)?,
    })
}

pub fn get_logged_op(conn: &Connection, id: &str) -> Result<Option<LoggedOp>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, source, destination, status, undo_data_json FROM action_log WHERE id = ?1",
    )?;
    Ok(stmt.query_row(params![id], row_to_logged_op).optional()?)
}

/// Most-recent operations that are still undoable (status = executed).
pub fn recent_undoable(conn: &Connection, limit: i64) -> Result<Vec<LoggedOp>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, source, destination, status, undo_data_json
         FROM action_log WHERE status = 'executed' ORDER BY executed_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], row_to_logged_op)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn mark_undone(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE action_log SET status = 'undone', undone_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;
    Ok(())
}

// ----------------------------------------------------------------------- trash

#[allow(clippy::too_many_arguments)]
pub fn insert_trash(
    conn: &Connection,
    original_path: &str,
    trash_path: &str,
    name: &str,
    size: u64,
    is_dir: bool,
    expires_at: &str,
) -> Result<TrashItem> {
    let item = TrashItem {
        id: Uuid::new_v4().to_string(),
        original_path: original_path.to_string(),
        trash_path: trash_path.to_string(),
        name: name.to_string(),
        size,
        is_dir,
        deleted_at: now(),
        expires_at: expires_at.to_string(),
        restored_at: None,
    };
    conn.execute(
        "INSERT INTO trash_items
           (id, original_path, trash_path, name, size, is_dir, deleted_at, expires_at, restored_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![
            item.id, item.original_path, item.trash_path, item.name,
            item.size as i64, item.is_dir as i64, item.deleted_at, item.expires_at
        ],
    )?;
    Ok(item)
}

fn row_to_trash(r: &Row) -> rusqlite::Result<TrashItem> {
    Ok(TrashItem {
        id: r.get(0)?,
        original_path: r.get(1)?,
        trash_path: r.get(2)?,
        name: r.get(3)?,
        size: r.get::<_, i64>(4)? as u64,
        is_dir: r.get::<_, i64>(5)? != 0,
        deleted_at: r.get(6)?,
        expires_at: r.get(7)?,
        restored_at: r.get(8)?,
    })
}

pub fn list_trash(conn: &Connection) -> Result<Vec<TrashItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, trash_path, name, size, is_dir, deleted_at, expires_at, restored_at
         FROM trash_items WHERE restored_at IS NULL ORDER BY deleted_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_trash)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_trash(conn: &Connection, id: &str) -> Result<Option<TrashItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, trash_path, name, size, is_dir, deleted_at, expires_at, restored_at
         FROM trash_items WHERE id = ?1",
    )?;
    Ok(stmt.query_row(params![id], row_to_trash).optional()?)
}

pub fn mark_restored(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE trash_items SET restored_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;
    Ok(())
}

pub fn delete_trash_row(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM trash_items WHERE id = ?1", params![id])?;
    Ok(())
}

/// Items whose retention window has elapsed (eligible for permanent cleanup).
pub fn expired_trash(conn: &Connection) -> Result<Vec<TrashItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, trash_path, name, size, is_dir, deleted_at, expires_at, restored_at
         FROM trash_items WHERE restored_at IS NULL AND expires_at <= ?1",
    )?;
    let rows = stmt.query_map(params![now()], row_to_trash)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ------------------------------------------------------------------ preferences

pub fn get_pref(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
    Ok(stmt
        .query_row(params![key], |r| r.get::<_, String>(0))
        .optional()?)
}

pub fn set_pref(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn all_prefs(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM preferences")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// -------------------------------------------------------------------- locations

pub fn record_location(
    conn: &Connection,
    path: &str,
    label: Option<&str>,
    kind: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO locations (id, path, label, kind, use_count, last_used)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)
         ON CONFLICT(path) DO UPDATE SET
            use_count = use_count + 1,
            last_used = excluded.last_used,
            label = COALESCE(excluded.label, locations.label)",
        params![Uuid::new_v4().to_string(), path, label, kind, now()],
    )?;
    Ok(())
}

pub fn list_locations(conn: &Connection, limit: i64) -> Result<Vec<Location>> {
    let mut stmt = conn.prepare(
        "SELECT path, label, kind, use_count, last_used
         FROM locations ORDER BY use_count DESC, last_used DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(Location {
            path: r.get(0)?,
            label: r.get(1)?,
            kind: r.get(2)?,
            use_count: r.get(3)?,
            last_used: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

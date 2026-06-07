//! The Undo system. Reverses logged operations using the `undo_data` recorded
//! at execution time. Moves are moved back; deletes are restored from Trash;
//! program launches cannot be undone.

use crate::db::{self, LoggedOp};
use crate::{fsutil, trash};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct UndoData {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    trash_id: Option<String>,
}

/// Undo a single logged action by id.
pub fn undo_one(conn: &Connection, log_id: &str) -> Result<()> {
    let op = db::get_logged_op(conn, log_id)?.ok_or_else(|| anyhow!("log entry not found"))?;
    if op.status != "executed" {
        return Err(anyhow!(
            "this action cannot be undone (status: {})",
            op.status
        ));
    }
    apply_undo(conn, &op)?;
    db::mark_undone(conn, &op.id)?;
    Ok(())
}

/// Undo the most recent undoable action. Returns its id, or None if none remain.
pub fn undo_last(conn: &Connection) -> Result<Option<String>> {
    let recent = db::recent_undoable(conn, 1)?;
    match recent.into_iter().next() {
        Some(op) => {
            apply_undo(conn, &op)?;
            db::mark_undone(conn, &op.id)?;
            Ok(Some(op.id))
        }
        None => Ok(None),
    }
}

/// Undo several actions by id. Returns how many succeeded.
pub fn undo_many(conn: &Connection, ids: &[String]) -> Result<usize> {
    let mut n = 0;
    for id in ids {
        if undo_one(conn, id).is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

fn apply_undo(conn: &Connection, op: &LoggedOp) -> Result<()> {
    let data: UndoData = match &op.undo_data {
        Some(s) => serde_json::from_str(s).map_err(|e| anyhow!("corrupt undo data: {e}"))?,
        None => return Err(anyhow!("no undo data available for '{}'", op.kind)),
    };
    match data.kind.as_str() {
        "move" => {
            let from = data.from.ok_or_else(|| anyhow!("missing 'from'"))?;
            let to = data.to.ok_or_else(|| anyhow!("missing 'to'"))?;
            let to_path = fsutil::unique_dest(Path::new(&to));
            fsutil::move_path(Path::new(&from), &to_path)?;
        }
        "delete" => {
            let trash_id = data.trash_id.ok_or_else(|| anyhow!("missing trash_id"))?;
            trash::restore(conn, &trash_id)?;
        }
        other => return Err(anyhow!("cannot undo an operation of type '{other}'")),
    }
    Ok(())
}

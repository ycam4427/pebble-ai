//! The AI Trash. User-facing "delete" never destroys data — it moves the target
//! into a per-item slot under the trash root and records it for restore. Files
//! are only ever permanently removed by retention cleanup or an explicit
//! "empty trash", which are the sole callers of `fsutil::permanently_delete`.

use crate::models::TrashItem;
use crate::{db, fsops, fsutil};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TrashConfig {
    pub root: PathBuf,
    pub retention_days: u64,
}

/// Move a path into the trash and record it. Returns the created trash entry.
pub fn move_to_trash(conn: &Connection, path: &Path, cfg: &TrashConfig) -> Result<TrashItem> {
    let md = std::fs::symlink_metadata(path)
        .map_err(|e| anyhow!("cannot access '{}': {e}", path.display()))?;
    let is_dir = md.is_dir();
    let (size, _count) = fsops::dir_stats(path);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("invalid path (no file name)"))?;

    // Each trashed item gets its own slot so names never collide.
    let slot = cfg.root.join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&slot)?;
    let dest = slot.join(&name);

    fsutil::move_path(path, &dest)?;

    let expires = (Utc::now() + Duration::days(cfg.retention_days as i64)).to_rfc3339();
    let item = db::insert_trash(
        conn,
        &path.display().to_string(),
        &dest.display().to_string(),
        &name,
        size,
        is_dir,
        &expires,
    )?;
    Ok(item)
}

/// Restore a trashed item to its original location (auto-renamed if the original
/// path is now occupied).
pub fn restore(conn: &Connection, id: &str) -> Result<TrashItem> {
    let item = db::get_trash(conn, id)?.ok_or_else(|| anyhow!("trash item not found"))?;
    if item.restored_at.is_some() {
        return Err(anyhow!("item was already restored"));
    }
    let original = PathBuf::from(&item.original_path);
    let target = fsutil::unique_dest(&original);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    fsutil::move_path(Path::new(&item.trash_path), &target)?;
    // Remove the now-empty slot.
    if let Some(slot) = Path::new(&item.trash_path).parent() {
        let _ = std::fs::remove_dir_all(slot);
    }
    db::mark_restored(conn, id)?;
    Ok(item)
}

/// Permanently empty the trash. Returns the number of items removed.
pub fn empty(conn: &Connection) -> Result<usize> {
    let items = db::list_trash(conn)?;
    let mut n = 0;
    for it in items {
        if let Some(slot) = Path::new(&it.trash_path).parent() {
            let _ = fsutil::permanently_delete(slot);
        }
        db::delete_trash_row(conn, &it.id)?;
        n += 1;
    }
    Ok(n)
}

/// Permanently delete a single trashed item.
pub fn delete_one(conn: &Connection, id: &str) -> Result<()> {
    let item = db::get_trash(conn, id)?.ok_or_else(|| anyhow!("trash item not found"))?;
    if let Some(slot) = Path::new(&item.trash_path).parent() {
        let _ = fsutil::permanently_delete(slot);
    }
    db::delete_trash_row(conn, id)?;
    Ok(())
}

/// Permanently remove items past their retention window. Returns count removed.
pub fn cleanup(conn: &Connection) -> Result<usize> {
    let expired = db::expired_trash(conn)?;
    let mut n = 0;
    for it in expired {
        if let Some(slot) = Path::new(&it.trash_path).parent() {
            let _ = fsutil::permanently_delete(slot);
        }
        db::delete_trash_row(conn, &it.id)?;
        n += 1;
    }
    Ok(n)
}

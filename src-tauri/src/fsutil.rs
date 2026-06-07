//! Low-level filesystem move/copy/delete helpers shared by the trash, executor,
//! and undo modules. Cross-device aware (falls back to copy+remove when a plain
//! rename can't span volumes).

use std::io;
use std::path::{Path, PathBuf};

/// Move a file or directory, creating parent directories as needed.
pub fn move_path(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    // rename failed (commonly: across volumes) — copy then remove.
    let md = std::fs::symlink_metadata(src)?;
    if md.is_dir() {
        copy_dir_all(src, dst)?;
        std::fs::remove_dir_all(src)?;
    } else {
        std::fs::copy(src, dst)?;
        std::fs::remove_file(src)?;
    }
    Ok(())
}

/// Recursively copy a directory tree.
pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Return `dst` if free, otherwise `name (1).ext`, `name (2).ext`, ... — never
/// overwrites an existing file.
pub fn unique_dest(dst: &Path) -> PathBuf {
    if !dst.exists() {
        return dst.to_path_buf();
    }
    let parent = dst.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = dst
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = dst.extension().map(|s| s.to_string_lossy().to_string());
    for i in 1..100_000 {
        let name = match &ext {
            Some(e) => format!("{stem} ({i}).{e}"),
            None => format!("{stem} ({i})"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    dst.to_path_buf()
}

/// Permanently remove a file or directory. Only ever called from Trash cleanup
/// / explicit "empty trash" — never from a user command directly.
pub fn permanently_delete(path: &Path) -> io::Result<()> {
    let md = std::fs::symlink_metadata(path)?;
    if md.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

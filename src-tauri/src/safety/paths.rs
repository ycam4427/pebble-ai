//! Path normalization and containment checks used by the validator.
//!
//! All comparisons are component-wise (so `C:\Windows` never matches
//! `C:\WindowsApps`) and case-insensitive on Windows.

use std::path::{Component, Path, PathBuf};

/// Lexically normalize a path (resolve `.` and `..`) without touching the disk.
/// Based on the well-known cargo implementation; handles the Windows prefix.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => ret.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => ret.push(c),
        }
    }
    ret
}

/// Make absolute (relative to CWD) then lexically normalize.
pub fn absolutize(path: &Path) -> std::io::Result<PathBuf> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize_path(&abs))
}

/// Resolve a path to an absolute, symlink-free form. For paths that do not yet
/// exist (e.g. a move destination), the deepest existing ancestor is canonicalized
/// (resolving any symlinks for safety) and the remaining tail re-appended.
pub fn resolve(path: &Path) -> std::io::Result<PathBuf> {
    let abs = absolutize(path)?;
    if abs.exists() {
        return dunce::canonicalize(&abs);
    }

    let mut prefix = abs.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if prefix.exists() {
            break;
        }
        let name = prefix.file_name().map(|s| s.to_os_string());
        let parent = prefix.parent().map(|p| p.to_path_buf());
        match (name, parent) {
            (Some(name), Some(parent)) => {
                tail.push(name);
                prefix = parent;
            }
            _ => break,
        }
    }

    let mut base = if prefix.exists() {
        dunce::canonicalize(&prefix)?
    } else {
        prefix
    };
    for name in tail.iter().rev() {
        base.push(name);
    }
    Ok(base)
}

fn comp_key(c: &Component) -> String {
    let s = c.as_os_str().to_string_lossy();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s.to_string()
    }
}

/// True if `child` is equal to or nested under `ancestor` (component-wise).
pub fn path_under(child: &Path, ancestor: &Path) -> bool {
    let a: Vec<String> = ancestor.components().map(|c| comp_key(&c)).collect();
    if a.is_empty() {
        return false;
    }
    let c: Vec<String> = child.components().map(|c| comp_key(&c)).collect();
    if a.len() > c.len() {
        return false;
    }
    a.iter().zip(c.iter()).all(|(x, y)| x == y)
}

/// True if two paths refer to the same location (component-wise compare).
pub fn paths_equal(a: &Path, b: &Path) -> bool {
    let av: Vec<String> = a.components().map(|c| comp_key(&c)).collect();
    let bv: Vec<String> = b.components().map(|c| comp_key(&c)).collect();
    av == bv
}

/// True if `path` is a filesystem/drive root (has no parent once normalized).
pub fn is_root(path: &Path) -> bool {
    path.parent().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_prefix_is_not_loose() {
        // C:\Windows must not be considered an ancestor of C:\WindowsApps.
        let parent = Path::new("C:\\Windows");
        assert!(path_under(Path::new("C:\\Windows\\System32"), parent));
        assert!(!path_under(Path::new("C:\\WindowsApps\\foo"), parent));
    }

    #[test]
    fn case_insensitive_on_windows() {
        if cfg!(windows) {
            assert!(path_under(
                Path::new("c:\\users\\bob\\downloads\\x.png"),
                Path::new("C:\\Users\\Bob")
            ));
        }
    }
}

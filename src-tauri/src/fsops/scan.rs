//! Filesystem scanners. All read-only.

use crate::models::{category_for, CategoryStat, DupGroup, FileEntry, FolderAnalysis, StorageStats};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Safety cap so a scan of an enormous tree can never run unbounded.
const MAX_ENTRIES: usize = 200_000;

fn systime_rfc(t: Option<SystemTime>) -> Option<String> {
    t.map(|t| {
        let dt: DateTime<Utc> = t.into();
        dt.to_rfc3339()
    })
}

fn to_entry(path: &Path, md: &Metadata) -> FileEntry {
    FileEntry {
        path: path.display().to_string(),
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        size: md.len(),
        is_dir: md.is_dir(),
        modified: systime_rfc(md.modified().ok()),
        accessed: systime_rfc(md.accessed().ok()),
        extension: path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase()),
    }
}

/// Largest files under `root`, descending by size.
pub fn find_large_files(root: &Path, min_bytes: u64, limit: usize) -> Result<Vec<FileEntry>> {
    let mut out: Vec<FileEntry> = Vec::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES {
            break;
        }
        if entry.file_type().is_file() {
            if let Ok(md) = entry.metadata() {
                if md.len() >= min_bytes {
                    out.push(to_entry(entry.path(), &md));
                    // Keep memory bounded for huge trees.
                    if out.len() > 4096 {
                        out.sort_by(|a, b| b.size.cmp(&a.size));
                        out.truncate(limit.max(512));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.size.cmp(&a.size));
    out.truncate(limit);
    Ok(out)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Groups of byte-identical files (matched by size, confirmed by SHA-256).
pub fn find_duplicates(root: &Path) -> Result<Vec<DupGroup>> {
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES {
            break;
        }
        if entry.file_type().is_file() {
            if let Ok(md) = entry.metadata() {
                if md.len() > 0 {
                    by_size.entry(md.len()).or_default().push(entry.into_path());
                }
            }
        }
    }

    let mut groups: Vec<DupGroup> = Vec::new();
    for (size, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        let mut by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for p in candidates {
            if let Ok(h) = hash_file(&p) {
                by_hash.entry(h).or_default().push(p);
            }
        }
        for (hash, paths) in by_hash {
            if paths.len() >= 2 {
                let files = paths
                    .iter()
                    .filter_map(|p| fs::metadata(p).ok().map(|md| to_entry(p, &md)))
                    .collect();
                groups.push(DupGroup { hash, size, files });
            }
        }
    }
    // Biggest reclaimable space first: size * (copies - 1).
    groups.sort_by(|a, b| {
        let wa = a.size * (a.files.len().saturating_sub(1)) as u64;
        let wb = b.size * (b.files.len().saturating_sub(1)) as u64;
        wb.cmp(&wa)
    });
    Ok(groups)
}

/// Files not accessed (fallback: not modified) within the last `days` days.
pub fn find_stale_files(root: &Path, days: u64, limit: usize) -> Result<Vec<FileEntry>> {
    let cutoff: DateTime<Utc> = Utc::now() - Duration::days(days as i64);
    let mut out: Vec<(DateTime<Utc>, FileEntry)> = Vec::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES {
            break;
        }
        if entry.file_type().is_file() {
            if let Ok(md) = entry.metadata() {
                let t = md.accessed().or_else(|_| md.modified()).ok();
                if let Some(t) = t {
                    let dt: DateTime<Utc> = t.into();
                    if dt < cutoff {
                        out.push((dt, to_entry(entry.path(), &md)));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0)); // oldest first
    Ok(out.into_iter().take(limit).map(|(_, e)| e).collect())
}

/// Extensions to treat as searchable text for content search.
fn is_texty(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .as_deref(),
        Some(
            "txt" | "md" | "markdown" | "rtf" | "csv" | "tsv" | "log" | "json" | "yaml" | "yml"
            | "toml" | "ini" | "cfg" | "conf" | "xml" | "html" | "htm" | "css" | "js" | "jsx"
            | "ts" | "tsx" | "rs" | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | "rb"
            | "php" | "sh" | "bat" | "ps1" | "sql"
        )
    )
}

/// Files whose *contents* contain `query` (case-insensitive). Text-like files
/// only, capped at 5 MB each; returns one snippet per matching file.
pub fn search_content(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<crate::models::ContentMatch>> {
    let needle = query.to_lowercase();
    if needle.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out: Vec<crate::models::ContentMatch> = Vec::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES || out.len() >= limit {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_texty(path) {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            if md.len() > 5_000_000 {
                continue;
            }
        }
        if let Ok(content) = fs::read_to_string(path) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    let snippet: String = line.trim().chars().take(160).collect();
                    out.push(crate::models::ContentMatch {
                        path: path.display().to_string(),
                        name: path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        line: (i + 1) as u64,
                        snippet,
                    });
                    break; // one match per file is enough for a result list
                }
            }
        }
    }
    Ok(out)
}

/// Files whose name contains `query` (case-insensitive).
pub fn search_files(root: &Path, query: &str, limit: usize) -> Result<Vec<FileEntry>> {
    let needle = query.to_lowercase();
    let mut out: Vec<FileEntry> = Vec::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES || out.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains(&needle) {
            if let Ok(md) = entry.metadata() {
                out.push(to_entry(entry.path(), &md));
            }
        }
    }
    Ok(out)
}

struct Acc {
    total_bytes: u64,
    file_count: u64,
    dir_count: u64,
    by_category: HashMap<&'static str, (u64, u64)>,
    by_extension: HashMap<String, (u64, u64)>,
    largest: Vec<FileEntry>,
    recent: Vec<(DateTime<Utc>, FileEntry)>,
    truncated: bool,
}

impl Acc {
    fn new() -> Self {
        Acc {
            total_bytes: 0,
            file_count: 0,
            dir_count: 0,
            by_category: HashMap::new(),
            by_extension: HashMap::new(),
            largest: Vec::new(),
            recent: Vec::new(),
            truncated: false,
        }
    }

    fn add_file(&mut self, path: &Path, md: &Metadata) {
        let size = md.len();
        self.total_bytes += size;
        self.file_count += 1;
        let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
        let cat = category_for(ext.as_deref());
        let e = self.by_category.entry(cat).or_insert((0, 0));
        e.0 += size;
        e.1 += 1;
        if let Some(ext) = &ext {
            let e = self.by_extension.entry(ext.clone()).or_insert((0, 0));
            e.0 += size;
            e.1 += 1;
        }
        let entry = to_entry(path, md);
        self.largest.push(entry.clone());
        if self.largest.len() > 256 {
            self.largest.sort_by(|a, b| b.size.cmp(&a.size));
            self.largest.truncate(32);
        }
        if let Ok(t) = md.modified() {
            let dt: DateTime<Utc> = t.into();
            self.recent.push((dt, entry));
            if self.recent.len() > 256 {
                self.recent.sort_by(|a, b| b.0.cmp(&a.0));
                self.recent.truncate(32);
            }
        }
    }

    fn categories(&self) -> Vec<CategoryStat> {
        let mut v: Vec<CategoryStat> = self
            .by_category
            .iter()
            .map(|(k, (bytes, count))| CategoryStat {
                category: k.to_string(),
                bytes: *bytes,
                count: *count,
            })
            .collect();
        v.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        v
    }

    fn extensions(&self, top: usize) -> Vec<CategoryStat> {
        let mut v: Vec<CategoryStat> = self
            .by_extension
            .iter()
            .map(|(k, (bytes, count))| CategoryStat {
                category: k.clone(),
                bytes: *bytes,
                count: *count,
            })
            .collect();
        v.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        v.truncate(top);
        v
    }

    fn largest(&self, top: usize) -> Vec<FileEntry> {
        let mut v = self.largest.clone();
        v.sort_by(|a, b| b.size.cmp(&a.size));
        v.truncate(top);
        v
    }

    fn recent(&self, top: usize) -> Vec<FileEntry> {
        let mut v = self.recent.clone();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.into_iter().take(top).map(|(_, e)| e).collect()
    }
}

fn walk_accumulate(root: &Path) -> Acc {
    let mut acc = Acc::new();
    let mut visited = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES {
            acc.truncated = true;
            break;
        }
        if entry.file_type().is_dir() {
            acc.dir_count += 1;
        } else if entry.file_type().is_file() {
            if let Ok(md) = entry.metadata() {
                acc.add_file(entry.path(), &md);
            }
        }
    }
    acc
}

/// Storage usage summary for a root.
pub fn storage_stats(root: &Path) -> Result<StorageStats> {
    let acc = walk_accumulate(root);
    Ok(StorageStats {
        root: root.display().to_string(),
        total_bytes: acc.total_bytes,
        file_count: acc.file_count,
        dir_count: acc.dir_count,
        by_category: acc.categories(),
        largest: acc.largest(10),
        truncated: acc.truncated,
    })
}

/// Detailed analysis of a single folder.
pub fn analyze_folder(root: &Path) -> Result<FolderAnalysis> {
    let acc = walk_accumulate(root);
    Ok(FolderAnalysis {
        root: root.display().to_string(),
        total_bytes: acc.total_bytes,
        file_count: acc.file_count,
        dir_count: acc.dir_count,
        by_category: acc.categories(),
        by_extension: acc.extensions(15),
        recent: acc.recent(10),
        truncated: acc.truncated,
    })
}

/// Immediate children of a directory (folders first, then files, by name).
pub fn list_dir(root: &Path) -> Result<Vec<FileEntry>> {
    let mut out: Vec<FileEntry> = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if let Ok(md) = entry.metadata() {
            out.push(to_entry(&entry.path(), &md));
        }
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// Total size + file count for a path (file => (len, 1)). Used by the planner to
/// preview folder deletes / bulk operations.
pub fn dir_stats(path: &Path) -> (u64, u64) {
    if let Ok(md) = fs::symlink_metadata(path) {
        if md.is_file() {
            return (md.len(), 1);
        }
    }
    let mut bytes = 0u64;
    let mut count = 0u64;
    let mut visited = 0usize;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        visited += 1;
        if visited > MAX_ENTRIES {
            break;
        }
        if entry.file_type().is_file() {
            if let Ok(md) = entry.metadata() {
                bytes += md.len();
                count += 1;
            }
        }
    }
    (bytes, count)
}

//! Action Planner. Converts abstract [`Action`]s (from the AI) into either:
//!   * a concrete [`QueryResult`] for read-only actions (run immediately), or
//!   * a list of concrete [`Operation`]s for mutating actions (sent to the
//!     Safety Validator before anything happens).
//!
//! The planner resolves user-friendly paths (`~`, `%DOWNLOADS%`, env vars) and
//! scans the filesystem to size operations for the confirmation preview.

use crate::models::{category_for, Action, OpKind, Operation, QueryResult};
use crate::fsops;
use crate::safety::paths::absolutize;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MB: u64 = 1024 * 1024;

/// How an action should be handled.
pub enum Class {
    /// Read-only — run via [`run_query`].
    Read,
    /// Mutating — expand via [`build_ops`], then validate.
    Mutate,
    /// Needs the LLM (document summary) — handled by the command layer.
    Summarize,
    /// Web search — handled by the command layer (network + opt-in).
    Web,
    /// Weather lookup — handled by the command layer (network + opt-in).
    Weather,
}

pub fn classify(action: &Action) -> Class {
    match action {
        Action::SummarizeDocument { .. } => Class::Summarize,
        Action::WebSearch { .. } => Class::Web,
        Action::GetWeather { .. } => Class::Weather,
        Action::FindLargeFiles { .. }
        | Action::FindDuplicates { .. }
        | Action::FindStaleFiles { .. }
        | Action::SearchFiles { .. }
        | Action::SearchContent { .. }
        | Action::StorageStats { .. }
        | Action::AnalyzeFolder { .. }
        | Action::ReadFile { .. }
        | Action::ReadImageText { .. } => Class::Read,
        _ => Class::Mutate,
    }
}

/// Execute a read-only action immediately and return its result card.
pub fn run_query(action: &Action) -> QueryResult {
    match action {
        Action::FindLargeFiles { root, min_mb, limit } => {
            let r = expand(root);
            let min = min_mb.unwrap_or(100).saturating_mul(MB);
            let lim = limit.unwrap_or(50).clamp(1, 1000);
            match fsops::find_large_files(&r, min, lim) {
                Ok(files) => QueryResult::LargeFiles {
                    root: r.display().to_string(),
                    files,
                },
                Err(e) => err(format!("Couldn't scan for large files: {e}")),
            }
        }
        Action::FindDuplicates { root } => {
            let r = expand(root);
            match fsops::find_duplicates(&r) {
                Ok(groups) => QueryResult::Duplicates {
                    root: r.display().to_string(),
                    groups,
                },
                Err(e) => err(format!("Couldn't scan for duplicates: {e}")),
            }
        }
        Action::FindStaleFiles { root, days } => {
            let r = expand(root);
            let d = days.unwrap_or(180);
            match fsops::find_stale_files(&r, d, 300) {
                Ok(files) => QueryResult::StaleFiles {
                    root: r.display().to_string(),
                    days: d,
                    files,
                },
                Err(e) => err(format!("Couldn't scan for stale files: {e}")),
            }
        }
        Action::SearchFiles { root, query } => {
            let trimmed = root.trim().to_lowercase();
            let broad = trimmed.is_empty()
                || matches!(
                    trimmed.as_str(),
                    "everywhere" | "anywhere" | "all" | "my computer" | "this pc" | "computer"
                );
            if broad {
                let files = search_broad(query, 400);
                QueryResult::SearchResults {
                    root: "everywhere".to_string(),
                    query: query.clone(),
                    files,
                }
            } else {
                let r = expand(root);
                match fsops::search_files(&r, query, 300) {
                    Ok(files) => QueryResult::SearchResults {
                        root: r.display().to_string(),
                        query: query.clone(),
                        files,
                    },
                    Err(e) => err(format!("Search failed: {e}")),
                }
            }
        }
        Action::SearchContent { root, query } => {
            let r = expand(root);
            match fsops::search_content(&r, query, 200) {
                Ok(matches) => QueryResult::ContentMatches {
                    root: r.display().to_string(),
                    query: query.clone(),
                    matches,
                },
                Err(e) => err(format!("Content search failed: {e}")),
            }
        }
        Action::StorageStats { root } => {
            let r = root
                .as_ref()
                .map(|s| expand(s))
                .or_else(dirs::home_dir)
                .unwrap_or_else(|| PathBuf::from("."));
            match fsops::storage_stats(&r) {
                Ok(stats) => QueryResult::Storage { stats },
                Err(e) => err(format!("Couldn't compute storage usage: {e}")),
            }
        }
        Action::AnalyzeFolder { root } => {
            let r = expand(root);
            match fsops::analyze_folder(&r) {
                Ok(analysis) => QueryResult::FolderAnalysis { analysis },
                Err(e) => err(format!("Couldn't analyze folder: {e}")),
            }
        }
        Action::ReadFile { path } => {
            let p = expand(path);
            match fsops::extract_text(&p, 16 * 1024) {
                Ok((preview, truncated)) => QueryResult::FileContent {
                    path: p.display().to_string(),
                    preview,
                    truncated,
                },
                Err(e) => err(format!("Couldn't read file: {e}")),
            }
        }
        Action::ReadImageText { path } => {
            let p = expand(path);
            match crate::ocr::image_text(&p.display().to_string()) {
                Ok(text) => QueryResult::FileContent {
                    path: p.display().to_string(),
                    preview: text,
                    truncated: false,
                },
                Err(e) => err(format!("Couldn't read image text: {e}")),
            }
        }
        _ => err("internal: not a read-only action".to_string()),
    }
}

/// Expand a mutating action into concrete operations (resolved + sized).
pub fn build_ops(action: &Action) -> Result<Vec<Operation>> {
    match action {
        Action::MoveFile { source, destination } => {
            let src = expand(source);
            let mut dest = expand(destination);
            // Moving *into* an existing folder keeps the original file name.
            if dest.is_dir() {
                if let Some(name) = src.file_name() {
                    dest = dest.join(name);
                }
            }
            Ok(vec![make_op(OpKind::Move, &src, Some(&dest))])
        }
        Action::RenameFile { source, new_name } => {
            let src = expand(source);
            let parent = src.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let dest = parent.join(new_name);
            Ok(vec![make_op(OpKind::Rename, &src, Some(&dest))])
        }
        Action::DeleteFile { path } | Action::DeleteFolder { path } => {
            let src = expand(path);
            Ok(vec![make_op(OpKind::Delete, &src, None)])
        }
        Action::OrganizeFolder { root, strategy } => {
            build_organize(&expand(root), strategy.as_deref().unwrap_or("by_type"))
        }
        Action::ClearFolder { root } => {
            let r = expand(root);
            let rd = std::fs::read_dir(&r)
                .map_err(|e| anyhow!("cannot read folder '{}': {e}", r.display()))?;
            let mut ops = Vec::new();
            for entry in rd.flatten() {
                ops.push(make_op(OpKind::Delete, &entry.path(), None));
            }
            Ok(ops)
        }
        Action::CleanDuplicates { root } => {
            let r = expand(root);
            let groups = fsops::find_duplicates(&r)
                .map_err(|e| anyhow!("couldn't scan for duplicates in '{}': {e}", r.display()))?;
            let mut ops = Vec::new();
            for g in groups {
                // keep the first copy; send the rest to the recoverable Trash
                for f in g.files.iter().skip(1) {
                    ops.push(make_op(OpKind::Delete, Path::new(&f.path), None));
                }
            }
            Ok(ops)
        }
        Action::ExecuteProgram { path, args } => {
            let src = expand(path);
            let mut op = make_op(OpKind::Execute, &src, None);
            op.args = args.clone();
            Ok(vec![op])
        }
        Action::EmptyRecycleBin => Ok(vec![Operation {
            id: Uuid::new_v4().to_string(),
            kind: OpKind::EmptyRecycleBin,
            source: "Windows Recycle Bin".to_string(),
            destination: None,
            size_bytes: 0,
            is_dir: false,
            file_count: 0,
            args: Vec::new(),
        }]),
        _ => Ok(vec![]),
    }
}

fn build_organize(root: &Path, strategy: &str) -> Result<Vec<Operation>> {
    let rd = std::fs::read_dir(root)
        .map_err(|e| anyhow!("cannot read folder '{}': {e}", root.display()))?;
    let mut ops = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !md.is_file() {
            continue; // organize only loose files, not existing subfolders
        }
        let name = match path.file_name() {
            Some(n) => n.to_owned(),
            None => continue,
        };
        let subfolder = if strategy == "by_date" {
            date_folder(&md)
        } else {
            category_for(path.extension().and_then(|e| e.to_str())).to_string()
        };
        let dest = root.join(&subfolder).join(&name);
        ops.push(make_op(OpKind::Move, &path, Some(&dest)));
    }
    Ok(ops)
}

fn date_folder(md: &Metadata) -> String {
    match md.modified() {
        Ok(t) => {
            let dt: DateTime<Utc> = t.into();
            dt.format("%Y-%m").to_string()
        }
        Err(_) => "undated".to_string(),
    }
}

fn make_op(kind: OpKind, src: &Path, dest: Option<&Path>) -> Operation {
    let (size, count) = fsops::dir_stats(src);
    let is_dir = std::fs::symlink_metadata(src)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    Operation {
        id: Uuid::new_v4().to_string(),
        kind,
        source: src.display().to_string(),
        destination: dest.map(|d| d.display().to_string()),
        size_bytes: size,
        is_dir,
        file_count: count,
        args: Vec::new(),
    }
}

fn err(message: String) -> QueryResult {
    QueryResult::Error { message }
}

// ---------------------------------------------------------------- path expansion

/// Public wrapper around path expansion (`~`, `%TOKENS%`, env vars).
pub fn resolve_path(input: &str) -> PathBuf {
    expand(input)
}

fn expand(input: &str) -> PathBuf {
    let mut s = input.trim().to_string();

    // Friendly folder tokens used in the prompt examples.
    for (token, dir) in [
        ("%HOME%", dirs::home_dir()),
        ("%DOWNLOADS%", dirs::download_dir()),
        ("%PICTURES%", dirs::picture_dir()),
        ("%DOCUMENTS%", dirs::document_dir()),
        ("%DESKTOP%", dirs::desktop_dir()),
        ("%VIDEOS%", dirs::video_dir()),
        ("%MUSIC%", dirs::audio_dir()),
    ] {
        if let Some(dir) = dir {
            s = replace_ci(&s, token, &dir.display().to_string());
        }
    }

    // Steam games convenience token used in the prompt examples.
    if s.to_ascii_uppercase().contains("%STEAM_COMMON%") {
        if let Some(sc) = steam_common() {
            s = replace_ci(&s, "%STEAM_COMMON%", &sc.display().to_string());
        }
    }

    // ~ -> home
    if let Some(rest) = s.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            let rest = rest.trim_start_matches(['/', '\\']);
            return abs(home.join(rest));
        }
    }

    // Remaining %VAR% -> environment values.
    s = expand_env_vars(&s);
    abs(PathBuf::from(s))
}

fn abs(p: PathBuf) -> PathBuf {
    absolutize(&p).unwrap_or(p)
}

/// ASCII-case-insensitive replace (tokens are ASCII; non-ASCII path bytes are
/// copied char-by-char so multibyte characters are never split).
fn replace_ci(haystack: &str, from: &str, to: &str) -> String {
    let hb = haystack.as_bytes();
    let fb = from.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < hb.len() {
        if i + fb.len() <= hb.len() && hb[i..i + fb.len()].eq_ignore_ascii_case(fb) {
            out.push_str(to);
            i += fb.len();
        } else if let Some(ch) = haystack[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

fn expand_env_vars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'%' {
            if let Some(end_rel) = s[i + 1..].find('%') {
                let var = &s[i + 1..i + 1 + end_rel];
                if let Ok(val) = std::env::var(var) {
                    out.push_str(&val);
                    i = i + 1 + end_rel + 1;
                    continue;
                }
            }
        }
        match s[i..].chars().next() {
            Some(ch) => {
                out.push(ch);
                i += ch.len_utf8();
            }
            None => break,
        }
    }
    out
}

/// First existing Steam `steamapps\common` library, for the %STEAM_COMMON% token.
fn steam_common() -> Option<PathBuf> {
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(p) = std::env::var(var) {
            let c = PathBuf::from(p).join("Steam").join("steamapps").join("common");
            if c.exists() {
                return Some(c);
            }
        }
    }
    for drive in ["C", "D", "E", "F"] {
        for name in ["Steam", "SteamLibrary"] {
            let c = PathBuf::from(format!("{drive}:\\{name}\\steamapps\\common"));
            if c.exists() {
                return Some(c);
            }
        }
    }
    None
}

/// Search several likely places (home, Program Files, Steam, other drives) and
/// merge the results — used when the user doesn't know where something is.
fn search_broad(query: &str, limit: usize) -> Vec<crate::models::FileEntry> {
    let mut out: Vec<crate::models::FileEntry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in search_roots() {
        if out.len() >= limit {
            break;
        }
        if let Ok(files) = fsops::search_files(&root, query, limit) {
            for f in files {
                if seen.insert(f.path.clone()) {
                    out.push(f);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    out
}

fn search_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(h) = dirs::home_dir() {
        roots.push(h);
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(p) = std::env::var(var) {
            let pb = PathBuf::from(p);
            if pb.exists() && !roots.contains(&pb) {
                roots.push(pb);
            }
        }
    }
    if let Some(sc) = steam_common() {
        if !roots.contains(&sc) {
            roots.push(sc);
        }
    }
    // Data/game drives (skip C: — already covered by home + Program Files).
    for letter in ['D', 'E', 'F', 'G'] {
        let p = PathBuf::from(format!("{letter}:\\"));
        if p.exists() {
            roots.push(p);
        }
    }
    roots
}

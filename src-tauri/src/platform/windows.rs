//! Windows path policy.

use super::PlatformPaths;
use std::path::PathBuf;

pub fn platform_paths() -> PlatformPaths {
    let mut protected: Vec<PathBuf> = Vec::new();

    // Prefer environment-derived locations (robust to non-C: installs / locales).
    for var in [
        "WINDIR",
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "ProgramData",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                protected.push(PathBuf::from(val));
            }
        }
    }

    // Hard-coded fallbacks (defense in depth if the environment is stripped).
    for p in [
        "C:\\Windows",
        "C:\\Program Files",
        "C:\\Program Files (x86)",
        "C:\\ProgramData",
    ] {
        protected.push(PathBuf::from(p));
    }

    // Treat the whole AppData tree as a system area — the MVP commands operate on
    // Downloads/Pictures/Documents/Desktop, never inside AppData.
    if let Some(home) = dirs::home_dir() {
        protected.push(home.join("AppData"));
    }

    // Sandbox: by default, mutations are only allowed under the user's home.
    // Extra roots (e.g. D:\) can be added by the user in Settings.
    let mut managed: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        managed.push(home);
    }

    PlatformPaths {
        protected,
        managed,
        known_folders: known_folders(),
        common_locations: common_locations(),
    }
}

/// Places where installed programs and games typically live — used to help the
/// assistant search for things like Steam games instead of only the home folder.
fn common_locations() -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();

    for (label, var) in [
        ("Program Files", "ProgramFiles"),
        ("Program Files (x86)", "ProgramFiles(x86)"),
    ] {
        if let Ok(p) = std::env::var(var) {
            let pb = PathBuf::from(p);
            if pb.exists() {
                v.push((label.to_string(), pb));
            }
        }
    }

    // Locate Steam installs, then their game libraries (games like Skyrim live in
    // <library>\steamapps\common).
    let mut steam_roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(p) = std::env::var(var) {
            let s = PathBuf::from(p).join("Steam");
            if s.exists() {
                steam_roots.push(s);
            }
        }
    }
    for drive in ["C", "D", "E", "F"] {
        for name in ["Steam", "SteamLibrary"] {
            let s = PathBuf::from(format!("{drive}:\\{name}"));
            if s.exists() && !steam_roots.iter().any(|x| x == &s) {
                steam_roots.push(s);
            }
        }
    }

    let mut commons: Vec<PathBuf> = Vec::new();
    for root in &steam_roots {
        let common = root.join("steamapps").join("common");
        if common.exists() && !commons.iter().any(|x| x == &common) {
            commons.push(common);
        }
        for vdf in [
            root.join("steamapps").join("libraryfolders.vdf"),
            root.join("config").join("libraryfolders.vdf"),
        ] {
            if let Ok(text) = std::fs::read_to_string(&vdf) {
                for lib in parse_vdf_paths(&text) {
                    let c = lib.join("steamapps").join("common");
                    if c.exists() && !commons.iter().any(|x| x == &c) {
                        commons.push(c);
                    }
                }
            }
        }
    }
    for (i, c) in commons.into_iter().enumerate() {
        let label = if i == 0 {
            "Steam games".to_string()
        } else {
            format!("Steam library {}", i + 1)
        };
        v.push((label, c));
    }

    v
}

/// Best-effort extraction of `"path" "..."` entries from a Steam libraryfolders.vdf.
fn parse_vdf_paths(text: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        for i in 0..parts.len() {
            if parts[i].eq_ignore_ascii_case("path") {
                if let Some(raw) = parts.get(i + 2) {
                    let p = raw.replace("\\\\", "\\");
                    if !p.trim().is_empty() {
                        out.push(PathBuf::from(p));
                    }
                }
            }
        }
    }
    out
}

fn known_folders() -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    if let Some(p) = dirs::home_dir() {
        v.push(("Home".into(), p));
    }
    if let Some(p) = dirs::download_dir() {
        v.push(("Downloads".into(), p));
    }
    if let Some(p) = dirs::document_dir() {
        v.push(("Documents".into(), p));
    }
    if let Some(p) = dirs::picture_dir() {
        v.push(("Pictures".into(), p));
    }
    if let Some(p) = dirs::desktop_dir() {
        v.push(("Desktop".into(), p));
    }
    if let Some(p) = dirs::video_dir() {
        v.push(("Videos".into(), p));
    }
    if let Some(p) = dirs::audio_dir() {
        v.push(("Music".into(), p));
    }
    v
}

//! Unix (Linux/macOS) path policy — scaffolding for future cross-platform support.
//! Not compiled on Windows. Fill out / refine when adding those targets.

use super::PlatformPaths;
use std::path::PathBuf;

pub fn platform_paths() -> PlatformPaths {
    // Covers both Linux and macOS roots; entries that don't exist simply never match.
    let protected: Vec<PathBuf> = [
        "/bin", "/sbin", "/usr", "/etc", "/boot", "/lib", "/lib64", "/opt", "/var", "/proc",
        "/sys", "/dev", // Linux
        "/System", "/Library", "/private", "/Applications", // macOS
    ]
    .iter()
    .map(PathBuf::from)
    .collect();

    let mut managed: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        managed.push(home);
    }

    let mut known_folders: Vec<(String, PathBuf)> = Vec::new();
    if let Some(p) = dirs::home_dir() {
        known_folders.push(("Home".into(), p));
    }
    if let Some(p) = dirs::download_dir() {
        known_folders.push(("Downloads".into(), p));
    }
    if let Some(p) = dirs::document_dir() {
        known_folders.push(("Documents".into(), p));
    }
    if let Some(p) = dirs::picture_dir() {
        known_folders.push(("Pictures".into(), p));
    }
    if let Some(p) = dirs::desktop_dir() {
        known_folders.push(("Desktop".into(), p));
    }

    let mut common_locations: Vec<(String, PathBuf)> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        for rel in [".steam/steam/steamapps/common", ".local/share/Steam/steamapps/common"] {
            let p = home.join(rel);
            if p.exists() {
                common_locations.push(("Steam games".into(), p));
            }
        }
    }

    PlatformPaths {
        protected,
        managed,
        known_folders,
        common_locations,
    }
}

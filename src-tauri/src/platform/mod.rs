//! Platform abstraction. Windows is implemented now; `unix` is wired up behind
//! `#[cfg(unix)]` so Linux/macOS support is a matter of fleshing it out — the
//! rest of the app depends only on the platform-neutral [`PlatformPaths`].

use std::path::PathBuf;

/// Platform-specific path policy used by the safety validator.
#[derive(Debug, Clone)]
pub struct PlatformPaths {
    /// Deny-list: prefixes the assistant must never modify.
    pub protected: Vec<PathBuf>,
    /// Allow-list (sandbox): the only roots under which mutations are permitted.
    pub managed: Vec<PathBuf>,
    /// Friendly user folders (label, path) for the UI and prompt context.
    pub known_folders: Vec<(String, PathBuf)>,
    /// Common non-home places where programs/games live (Steam, Program Files…),
    /// so the assistant knows where to search for installed apps.
    pub common_locations: Vec<(String, PathBuf)>,
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::platform_paths;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::platform_paths;

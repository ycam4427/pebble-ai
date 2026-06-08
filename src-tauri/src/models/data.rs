//! Read-only data structures returned by Tier-0 filesystem analysis.

use serde::{Deserialize, Serialize};

/// A single file or directory entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    /// RFC3339 timestamps (None if unavailable on the platform).
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub extension: Option<String>,
}

/// Aggregated size/count for a category or extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    pub category: String,
    pub bytes: u64,
    pub count: u64,
}

/// Storage usage summary for a root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub root: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub by_category: Vec<CategoryStat>,
    pub largest: Vec<FileEntry>,
    /// True if the scan was capped (very large tree) and numbers are partial.
    pub truncated: bool,
}

/// A single web search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// A single in-file content match (for the Content Search extension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMatch {
    pub path: String,
    pub name: String,
    pub line: u64,
    pub snippet: String,
}

/// Current weather (from wttr.in — opt-in, uses the internet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub location: String,
    pub temp_c: String,
    pub temp_f: String,
    pub feels_like_c: String,
    pub description: String,
    pub humidity: String,
    pub wind_kmph: String,
}

/// A group of byte-for-byte identical files (same size + same SHA-256).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DupGroup {
    pub hash: String,
    pub size: u64,
    pub files: Vec<FileEntry>,
}

/// Detailed analysis of a single folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderAnalysis {
    pub root: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub by_category: Vec<CategoryStat>,
    pub by_extension: Vec<CategoryStat>,
    pub recent: Vec<FileEntry>,
    pub truncated: bool,
}

/// The result of a Tier-0 query, tagged for the frontend to render as a card.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryResult {
    LargeFiles {
        root: String,
        files: Vec<FileEntry>,
    },
    Duplicates {
        root: String,
        groups: Vec<DupGroup>,
    },
    StaleFiles {
        root: String,
        days: u64,
        files: Vec<FileEntry>,
    },
    SearchResults {
        root: String,
        query: String,
        files: Vec<FileEntry>,
    },
    ContentMatches {
        root: String,
        query: String,
        matches: Vec<ContentMatch>,
    },
    Storage {
        stats: StorageStats,
    },
    FolderAnalysis {
        analysis: FolderAnalysis,
    },
    FileContent {
        path: String,
        preview: String,
        truncated: bool,
    },
    Summary {
        path: String,
        summary: String,
    },
    WebResults {
        query: String,
        results: Vec<WebResult>,
    },
    Weather {
        info: WeatherInfo,
    },
    /// A read-only action that could not run (e.g. path rejected by safety).
    Error {
        message: String,
    },
}

/// Categorize a file by extension for storage breakdowns.
pub fn category_for(ext: Option<&str>) -> &'static str {
    match ext.map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "svg" | "tiff") => "Images",
        Some("mp4" | "mkv" | "mov" | "avi" | "wmv" | "flv" | "webm" | "m4v") => "Videos",
        Some("mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma") => "Audio",
        Some("pdf" | "doc" | "docx" | "txt" | "md" | "rtf" | "odt" | "ppt" | "pptx" | "xls"
        | "xlsx" | "csv") => "Documents",
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso") => "Archives",
        Some("exe" | "msi" | "dmg" | "deb" | "rpm" | "appimage") => "Installers",
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "java" | "c" | "cpp" | "h" | "go"
        | "rb" | "php" | "html" | "css" | "json" | "toml" | "yaml" | "yml" | "sh") => "Code",
        _ => "Other",
    }
}

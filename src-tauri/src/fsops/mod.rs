//! Read-only filesystem analysis (Tier 0) and text extraction.
//!
//! Nothing in this module mutates the filesystem. These are the operations the
//! assistant may run automatically (after a path safety check) to answer
//! questions like "find large files" or "show storage usage".

pub mod scan;
pub mod summarize;

pub use scan::{
    analyze_folder, dir_stats, find_duplicates, find_large_files, find_stale_files, list_dir,
    search_files, storage_stats,
};
pub use summarize::extract_text;

//! Text extraction for read/summarize actions. Plain-text/code/markdown only;
//! binary formats (pdf/docx/…) are intentionally out of scope for the MVP and
//! would be added via a future "Document Indexing"/"OCR" plugin.

use anyhow::{bail, Result};
use std::path::Path;

const TEXT_EXTS: &[&str] = &[
    "txt", "md", "markdown", "csv", "tsv", "log", "json", "toml", "yaml", "yml", "xml", "ini",
    "cfg", "conf", "rs", "ts", "tsx", "js", "jsx", "py", "java", "kt", "c", "cpp", "cc", "h", "hpp",
    "go", "rb", "php", "html", "css", "scss", "sh", "bat", "ps1", "sql",
];

/// Read up to `max_bytes` of a text file. Returns (content, truncated).
pub fn extract_text(path: &Path, max_bytes: usize) -> Result<(String, bool)> {
    let md = std::fs::metadata(path)?;
    if !md.is_file() {
        bail!("not a file");
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    let is_text = ext
        .as_deref()
        .map(|e| TEXT_EXTS.contains(&e))
        .unwrap_or(false);
    if !is_text {
        bail!(
            "unsupported file type ({}). Text, code, markdown and CSV are supported.",
            ext.unwrap_or_else(|| "no extension".to_string())
        );
    }

    let bytes = std::fs::read(path)?;
    let truncated = bytes.len() > max_bytes;
    let slice = &bytes[..bytes.len().min(max_bytes)];
    Ok((String::from_utf8_lossy(slice).to_string(), truncated))
}

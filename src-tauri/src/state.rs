//! Application state and configuration.
//!
//! `AppState` owns the DB connection, the live config, the platform path policy,
//! the Ollama client, and the map of *pending* validated plans (kept server-side
//! so a validated plan never round-trips through the untrusted frontend).

use crate::ai::OllamaClient;
use crate::platform::{self, PlatformPaths};
use crate::safety::SafetyConfig;
use crate::trash::TrashConfig;
use crate::{db, safety::ValidatedPlan};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub const DEFAULT_MODEL: &str = "llama3.2:3b";
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
pub const DEFAULT_RETENTION_DAYS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownFolder {
    pub label: String,
    pub path: String,
}

/// Serializable configuration shown in Settings and persisted to `preferences`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub ollama_url: String,
    pub retention_days: u64,
    pub allow_execute: bool,
    /// Whether Pebble may search the web (off by default; opt-in for privacy).
    pub allow_web: bool,
    /// Opt-in extensions (off by default).
    pub ext_content_search: bool,
    pub ext_ocr: bool,
    pub ext_dedupe: bool,
    /// Opt-in abilities.
    pub allow_weather: bool,
    /// Whether Pebble keeps a long-term memory about the user (he curates it).
    pub allow_memory: bool,
    /// Whether Pebble mirrors the user's tone/energy (on by default).
    pub adapt_tone: bool,
    /// Extra sandbox roots the user has opted into (beyond home).
    pub managed_roots: Vec<String>,
    // ---- personalization ----
    /// What Pebble calls the user.
    pub user_name: String,
    /// How Pebble behaves: "cozy" | "cheerful" | "calm" | "playful".
    pub persona: String,
    /// Optional free-text the user shared about themselves.
    pub about_you: String,
    /// Active UI theme key.
    pub theme: String,
    /// Whether the first-run welcome has been completed.
    pub onboarded: bool,
    // ---- display-only (derived) ----
    /// App version (compile-time), shown in Settings / About.
    pub app_version: String,
    pub app_root: String,
    pub trash_root: String,
    pub db_path: String,
    pub protected_roots: Vec<String>,
    pub known_folders: Vec<KnownFolder>,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub config: Mutex<Config>,
    pub platform: PlatformPaths,
    pub app_root: PathBuf,
    pub pending: Mutex<HashMap<String, ValidatedPlan>>,
    pub ollama: Mutex<OllamaClient>,
    /// Set true to ask an in-flight streaming generation to stop (Stop button).
    pub cancel: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let app_root = resolve_app_root();
        std::fs::create_dir_all(app_root.join("Trash"))?;
        let db_path = app_root.join("data").join("assistant.db");
        let conn = db::open(&db_path)?;
        let platform = platform::platform_paths();
        let config = load_config(&conn, &platform, &app_root, &db_path);
        let ollama = OllamaClient::new(&config.ollama_url);

        Ok(Self {
            db: Mutex::new(conn),
            config: Mutex::new(config),
            platform,
            app_root,
            pending: Mutex::new(HashMap::new()),
            ollama: Mutex::new(ollama),
            cancel: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Build the (independent) safety policy from platform paths + live config.
    pub fn safety_config(&self, cfg: &Config) -> SafetyConfig {
        let mut managed = self.platform.managed.clone();
        for m in &cfg.managed_roots {
            managed.push(PathBuf::from(m));
        }
        SafetyConfig {
            protected: self.platform.protected.clone(),
            managed,
            app_root: self.app_root.clone(),
            allow_execute: cfg.allow_execute,
        }
    }

    pub fn trash_config(&self, cfg: &Config) -> TrashConfig {
        TrashConfig {
            root: self.app_root.join("Trash"),
            retention_days: cfg.retention_days,
        }
    }
}

/// Choose the data root: `C:\AI_Assistant` when writable, else a per-user fallback.
fn resolve_app_root() -> PathBuf {
    #[cfg(windows)]
    {
        let primary = PathBuf::from("C:\\AI_Assistant");
        if std::fs::create_dir_all(&primary).is_ok() {
            return primary;
        }
    }
    if let Some(local) = dirs::data_local_dir() {
        let p = local.join("AI_Assistant");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    PathBuf::from("AI_Assistant")
}

fn load_config(
    conn: &Connection,
    platform: &PlatformPaths,
    app_root: &Path,
    db_path: &Path,
) -> Config {
    let get = |k: &str| db::get_pref(conn, k).ok().flatten();

    Config {
        model: get("model").unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        ollama_url: get("ollama_url").unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
        retention_days: get("retention_days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETENTION_DAYS),
        allow_execute: get("allow_execute").map(|s| s == "true").unwrap_or(false),
        allow_web: get("allow_web").map(|s| s == "true").unwrap_or(false),
        ext_content_search: get("ext_content_search").map(|s| s == "true").unwrap_or(false),
        ext_ocr: get("ext_ocr").map(|s| s == "true").unwrap_or(false),
        ext_dedupe: get("ext_dedupe").map(|s| s == "true").unwrap_or(false),
        allow_weather: get("allow_weather").map(|s| s == "true").unwrap_or(false),
        allow_memory: get("allow_memory").map(|s| s == "true").unwrap_or(false),
        adapt_tone: get("adapt_tone").map(|s| s == "true").unwrap_or(true),
        managed_roots: get("managed_roots")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        user_name: get("user_name").unwrap_or_default(),
        persona: get("persona").unwrap_or_else(|| "cozy".to_string()),
        about_you: get("about_you").unwrap_or_default(),
        theme: get("theme").unwrap_or_else(|| "pebble".to_string()),
        onboarded: get("onboarded").map(|s| s == "true").unwrap_or(false),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        app_root: app_root.display().to_string(),
        trash_root: app_root.join("Trash").display().to_string(),
        db_path: db_path.display().to_string(),
        protected_roots: platform
            .protected
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        known_folders: platform
            .known_folders
            .iter()
            .map(|(l, p)| KnownFolder {
                label: l.clone(),
                path: p.display().to_string(),
            })
            .collect(),
    }
}

/// Persist the mutable settings of a config.
pub fn persist_config(conn: &Connection, cfg: &Config) -> anyhow::Result<()> {
    db::set_pref(conn, "model", &cfg.model)?;
    db::set_pref(conn, "ollama_url", &cfg.ollama_url)?;
    db::set_pref(conn, "retention_days", &cfg.retention_days.to_string())?;
    db::set_pref(conn, "allow_execute", &cfg.allow_execute.to_string())?;
    db::set_pref(conn, "allow_web", &cfg.allow_web.to_string())?;
    db::set_pref(conn, "ext_content_search", &cfg.ext_content_search.to_string())?;
    db::set_pref(conn, "ext_ocr", &cfg.ext_ocr.to_string())?;
    db::set_pref(conn, "ext_dedupe", &cfg.ext_dedupe.to_string())?;
    db::set_pref(conn, "allow_weather", &cfg.allow_weather.to_string())?;
    db::set_pref(conn, "allow_memory", &cfg.allow_memory.to_string())?;
    db::set_pref(conn, "adapt_tone", &cfg.adapt_tone.to_string())?;
    db::set_pref(
        conn,
        "managed_roots",
        &serde_json::to_string(&cfg.managed_roots).unwrap_or_else(|_| "[]".to_string()),
    )?;
    db::set_pref(conn, "user_name", &cfg.user_name)?;
    db::set_pref(conn, "persona", &cfg.persona)?;
    db::set_pref(conn, "about_you", &cfg.about_you)?;
    db::set_pref(conn, "theme", &cfg.theme)?;
    db::set_pref(conn, "onboarded", &cfg.onboarded.to_string())?;
    Ok(())
}

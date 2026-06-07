#![allow(dead_code)] // future extension surface — intentionally unused in the MVP
//! Plugin architecture for future expansion.
//!
//! No capabilities are implemented yet — this defines the extension surface so
//! features like calendar, email, browser automation, OCR, document indexing,
//! a personal knowledge base, a voice assistant, and smart-home control can be
//! added later without touching the trusted core. A plugin can only *propose*
//! actions; everything still flows through the Safety Validator.

use crate::models::Action;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub enabled: bool,
}

/// The trait every future plugin implements.
pub trait Plugin: Send + Sync {
    fn info(&self) -> PluginInfo;

    /// Names of any extra actions this plugin understands.
    fn capabilities(&self) -> Vec<String> {
        Vec::new()
    }

    /// Handle a plugin-specific action. Plugins never touch the disk directly —
    /// they return [`Action`]s for the validator, or perform read-only work.
    fn propose(&self, _input: &str) -> anyhow::Result<Vec<Action>> {
        Ok(Vec::new())
    }
}

/// Registry of active plugins. Empty in the MVP.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn active(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|p| p.info()).collect()
    }

    /// The roadmap, surfaced in the UI as "coming soon".
    pub fn roadmap() -> Vec<PluginInfo> {
        [
            ("calendar", "Calendar Management", "Read and organize your calendar"),
            ("email", "Email Management", "Triage and summarize email"),
            ("browser", "Browser Automation", "Drive the browser for routine tasks"),
            ("coding", "Coding Assistant", "Project-aware coding help"),
            ("ocr", "OCR", "Extract text from images and scans"),
            ("indexing", "Document Indexing", "Fast local full-text search"),
            ("kb", "Personal Knowledge Base", "Your private, local knowledge store"),
            ("voice", "Voice Assistant", "Hands-free local voice control"),
            ("smarthome", "Smart Home Integration", "Control local smart devices"),
        ]
        .iter()
        .map(|(id, name, desc)| PluginInfo {
            id: (*id).to_string(),
            name: (*name).to_string(),
            description: (*desc).to_string(),
            version: "0.0.0".to_string(),
            enabled: false,
        })
        .collect()
    }
}

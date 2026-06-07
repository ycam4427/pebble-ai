//! AI integration layer (Ollama). This module can *suggest* actions only; it has
//! no path into the executor that bypasses the safety validator.

pub mod ollama;
pub mod prompt;
pub mod web;

pub use ollama::{ChatMessage, ModelStatus, OllamaClient};
pub use prompt::{planning_prompt, system_prompt};

//! The Safety Validator — the independent gate every operation must pass.
//!
//! This module imports `models` (plain data) but deliberately knows **nothing**
//! about the `ai`, `intent`, or `planner` modules. It is a pure function of
//! (operations, configuration). That independence is the whole point: even if
//! the AI, the prompt, or the planner is compromised or buggy, this layer is the
//! final, separate authority on what may touch the disk.

pub mod paths;
pub mod rules;
pub mod validator;

use crate::models::{Tier, ValidatedOp};
use serde::Serialize;
use std::path::PathBuf;

pub use validator::validate;

/// The inputs the validator needs, derived from the app `Config`.
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// Deny-list prefixes (system locations) — never modifiable.
    pub protected: Vec<PathBuf>,
    /// Allow-list (sandbox) roots — mutations only permitted underneath these.
    pub managed: Vec<PathBuf>,
    /// The assistant's own data directory — self-protected.
    pub app_root: PathBuf,
    /// Whether Tier-2 program execution is permitted at all (off by default).
    pub allow_execute: bool,
}

/// A plan that has passed the Safety Validator.
///
/// # Security invariant
///
/// All fields are private and the **only** constructor is
/// [`validator::validate`], which lives in a child module of `safety` (and so
/// may build this struct, while no outside module can). The executor's public
/// entry point accepts `&ValidatedPlan` by reference — it is therefore
/// impossible, even by mistake, to execute a plan that did not originate from
/// the validator. There is no `From<Plan>`, no `pub` fields, and no `new()`.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedPlan {
    id: String,
    summary: String,
    ops: Vec<ValidatedOp>,
    max_tier: Tier,
    affected_locations: Vec<String>,
    requires_typed_confirmation: bool,
    confirmation_phrase: Option<String>,
    move_count: usize,
    rename_count: usize,
    delete_count: usize,
    execute_count: usize,
    rejected_count: usize,
    warnings: Vec<String>,
    rejected: Vec<String>,
}

#[allow(dead_code)] // several accessors are consumed by the frontend via serde, not Rust
impl ValidatedPlan {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn summary(&self) -> &str {
        &self.summary
    }
    pub fn max_tier(&self) -> Tier {
        self.max_tier
    }
    pub fn ops(&self) -> &[ValidatedOp] {
        &self.ops
    }
    pub fn requires_typed_confirmation(&self) -> bool {
        self.requires_typed_confirmation
    }
    pub fn confirmation_phrase(&self) -> Option<&str> {
        self.confirmation_phrase.as_deref()
    }
    pub fn approved_ops(&self) -> impl Iterator<Item = &ValidatedOp> {
        self.ops.iter().filter(|o| o.is_approved())
    }
    pub fn has_approved(&self) -> bool {
        self.ops.iter().any(|o| o.is_approved())
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Verify a user-supplied confirmation phrase for a typed high-risk plan.
    pub fn confirmation_satisfied(&self, typed: Option<&str>) -> bool {
        match &self.confirmation_phrase {
            None => true,
            Some(phrase) => typed.map(|t| t.trim() == phrase).unwrap_or(false),
        }
    }
}

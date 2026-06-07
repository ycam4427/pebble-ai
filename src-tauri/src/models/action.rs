//! The action model — the *only* thing the AI is permitted to emit.
//!
//! Flow:  AI JSON  ->  [`Action`]  ->  (planner)  ->  [`Operation`]  ->
//!        (safety validator)  ->  `ValidatedOp` inside a `ValidatedPlan`.

use serde::{Deserialize, Serialize};

/// Permission tier for an operation. The validator assigns these; the AI never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Tier 0 — read-only analysis. Runs automatically after validation.
    Auto,
    /// Tier 1 — mutating, requires a simple confirmation.
    Confirm,
    /// Tier 2 — high-risk, requires explicit (sometimes typed) confirmation.
    HighRisk,
}

impl Tier {
    pub fn level(self) -> u8 {
        match self {
            Tier::Auto => 0,
            Tier::Confirm => 1,
            Tier::HighRisk => 2,
        }
    }

    #[allow(dead_code)]
    pub fn from_level(level: u8) -> Tier {
        match level {
            0 => Tier::Auto,
            1 => Tier::Confirm,
            _ => Tier::HighRisk,
        }
    }

    /// The more restrictive of two tiers.
    pub fn max(self, other: Tier) -> Tier {
        if self.level() >= other.level() {
            self
        } else {
            other
        }
    }
}

/// A natural-language intent, decoded from the model's structured JSON.
///
/// This is the complete, closed set of things the assistant can *propose*.
/// Note what is absent: there is no "run arbitrary code" or "raw shell" variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    // ---- Tier 0: read-only analysis -------------------------------------
    FindLargeFiles {
        root: String,
        #[serde(default)]
        min_mb: Option<u64>,
        #[serde(default)]
        limit: Option<usize>,
    },
    FindDuplicates {
        root: String,
    },
    FindStaleFiles {
        root: String,
        #[serde(default)]
        days: Option<u64>,
    },
    SearchFiles {
        root: String,
        query: String,
    },
    StorageStats {
        #[serde(default)]
        root: Option<String>,
    },
    AnalyzeFolder {
        root: String,
    },
    ReadFile {
        path: String,
    },
    SummarizeDocument {
        path: String,
    },
    /// Search the web (DuckDuckGo). Off unless enabled in Settings.
    WebSearch {
        query: String,
    },

    // ---- Tier 1: mutations (require confirmation) -----------------------
    MoveFile {
        source: String,
        destination: String,
    },
    RenameFile {
        source: String,
        new_name: String,
    },
    /// Bulk organize a folder, e.g. group loose files into category subfolders.
    OrganizeFolder {
        root: String,
        #[serde(default)]
        strategy: Option<String>, // "by_type" (default) | "by_date"
    },
    /// Clean/clear/empty a folder: move everything inside it to the recoverable
    /// Trash, keeping the folder itself.
    ClearFolder {
        root: String,
    },

    // ---- Tier 2: high-risk ---------------------------------------------
    DeleteFile {
        path: String,
    },
    DeleteFolder {
        path: String,
    },
    ExecuteProgram {
        path: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Empty the Windows Recycle Bin (permanent — separate from the AI Trash).
    EmptyRecycleBin,
}

/// The top-level object the model must return on every turn.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiPlan {
    /// Natural-language message shown to the user in the chat transcript.
    #[serde(default)]
    pub message: String,
    /// Zero or more proposed actions.
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// The kind of a concrete, executable operation. Distinct from [`Action`]:
/// the planner has already resolved paths, scanned the filesystem, and
/// expanded bulk intents into individual operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    Move,
    Rename,
    Delete,
    Execute,
    EmptyRecycleBin,
}

impl OpKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OpKind::Move => "move",
            OpKind::Rename => "rename",
            OpKind::Delete => "delete",
            OpKind::Execute => "execute",
            OpKind::EmptyRecycleBin => "empty_recycle_bin",
        }
    }
}

/// A single concrete, fully-resolved mutation. The executor only ever acts on
/// these — and only after the safety validator wraps them in a `ValidatedPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub kind: OpKind,
    /// Absolute, normalized source path.
    pub source: String,
    /// Absolute destination (move/rename); `None` for delete/execute.
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub is_dir: bool,
    /// For folder operations: number of files contained (for preview/threshold).
    #[serde(default)]
    pub file_count: u64,
    /// Arguments for an execute operation.
    #[serde(default)]
    pub args: Vec<String>,
}

/// The validator's decision for a single operation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Verdict {
    Approved,
    Rejected { reason: String },
}

/// An operation paired with the validator's verdict, tier, and any warnings.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedOp {
    pub op: Operation,
    pub tier: Tier,
    pub verdict: Verdict,
    pub warnings: Vec<String>,
}

impl ValidatedOp {
    pub fn is_approved(&self) -> bool {
        matches!(self.verdict, Verdict::Approved)
    }
}

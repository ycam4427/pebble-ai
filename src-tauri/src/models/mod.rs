//! Domain models shared across the trusted core.
//!
//! These types are deliberately free of any dependency on the `ai`, `intent`,
//! or `planner` modules. The AI is only ever allowed to produce [`Action`]
//! values (pure data); everything downstream operates on the concrete
//! [`Operation`] type that the planner builds and the safety validator blesses.

pub mod action;
pub mod data;
pub mod message;
pub mod records;

pub use action::{Action, AiPlan, OpKind, Operation, Tier, ValidatedOp, Verdict};
pub use data::{
    category_for, CategoryStat, DupGroup, FileEntry, FolderAnalysis, QueryResult, StorageStats,
    WebResult,
};
pub use message::{Conversation, GenStats, Message};
pub use records::{ActionLogEntry, Location, TrashItem};

//! Tier assignment and escalation thresholds. Pure policy, no I/O.

use crate::models::{OpKind, Tier};

/// More than this many operations in one plan escalates the whole plan to high-risk.
pub const BULK_THRESHOLD: usize = 100;

/// Deleting more than this many files requires a typed confirmation phrase.
pub const TYPED_CONFIRM_FILE_COUNT: u64 = 1000;

/// The base tier for an operation kind (before escalation).
pub fn tier_for(kind: OpKind) -> Tier {
    match kind {
        OpKind::Move | OpKind::Rename => Tier::Confirm,
        OpKind::Delete | OpKind::Execute | OpKind::EmptyRecycleBin => Tier::HighRisk,
    }
}

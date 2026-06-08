//! The validation engine. `validate` is the sole constructor of [`ValidatedPlan`].

use super::{paths, rules, SafetyConfig, ValidatedPlan};
use crate::models::{OpKind, Operation, Tier, ValidatedOp, Verdict};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Validate a list of concrete operations against the safety policy and produce
/// a sealed [`ValidatedPlan`]. This is the *only* place a `ValidatedPlan` is born.
pub fn validate(summary: String, ops: Vec<Operation>, cfg: &SafetyConfig) -> ValidatedPlan {
    let validated: Vec<ValidatedOp> = ops.into_iter().map(|op| validate_op(op, cfg)).collect();

    let mut move_count = 0usize;
    let mut rename_count = 0usize;
    let mut delete_count = 0usize;
    let mut execute_count = 0usize;
    let mut delete_files: u64 = 0;
    let mut has_empty_recycle = false;
    let mut max_tier = Tier::Auto;
    let mut locations: Vec<String> = Vec::new();

    for vo in validated.iter().filter(|o| o.is_approved()) {
        max_tier = max_tier.max(vo.tier);
        match vo.op.kind {
            OpKind::Move => move_count += 1,
            OpKind::Rename => rename_count += 1,
            OpKind::Delete => {
                delete_count += 1;
                delete_files += if vo.op.is_dir {
                    vo.op.file_count.max(1)
                } else {
                    1
                };
            }
            OpKind::Execute => execute_count += 1,
            OpKind::EmptyRecycleBin => has_empty_recycle = true,
        }
        collect_location(&mut locations, &vo.op.source);
        if let Some(dest) = &vo.op.destination {
            collect_location(&mut locations, dest);
        }
    }

    let rejected: Vec<String> = validated
        .iter()
        .filter_map(|o| match &o.verdict {
            Verdict::Rejected { reason } => Some(format!("{} — {}", short(&o.op.source), reason)),
            Verdict::Approved => None,
        })
        .collect();

    let approved_total = move_count + rename_count + delete_count + execute_count;
    let mut warnings: Vec<String> = Vec::new();

    // Escalation: a large batch becomes high-risk regardless of op kind.
    if approved_total > rules::BULK_THRESHOLD {
        max_tier = max_tier.max(Tier::HighRisk);
        warnings.push(format!(
            "Bulk operation: {} items will be affected.",
            approved_total
        ));
    }

    // Typed-confirmation gate for very large deletions.
    let requires_typed = delete_files > rules::TYPED_CONFIRM_FILE_COUNT || has_empty_recycle;
    let confirmation_phrase = if has_empty_recycle {
        Some("EMPTY RECYCLE BIN".to_string())
    } else if requires_typed {
        Some("CONFIRM DELETE".to_string())
    } else {
        None
    };
    if delete_files > rules::TYPED_CONFIRM_FILE_COUNT {
        warnings.push(format!(
            "High-risk: {} files will be moved to the Trash. Type the phrase to proceed.",
            delete_files
        ));
    }
    if has_empty_recycle {
        warnings.push(
            "This permanently empties the Windows Recycle Bin and cannot be undone.".to_string(),
        );
    }
    if execute_count > 0 {
        warnings.push("This plan launches external program(s).".to_string());
    }
    if !rejected.is_empty() {
        warnings.push(format!(
            "{} proposed action(s) were blocked by the Safety Validator.",
            rejected.len()
        ));
    }

    ValidatedPlan {
        id: Uuid::new_v4().to_string(),
        summary,
        ops: validated,
        max_tier,
        affected_locations: locations,
        requires_typed_confirmation: requires_typed,
        confirmation_phrase,
        move_count,
        rename_count,
        delete_count,
        execute_count,
        rejected_count: rejected.len(),
        warnings,
        rejected,
    }
}

fn validate_op(op: Operation, cfg: &SafetyConfig) -> ValidatedOp {
    let tier = rules::tier_for(op.kind);
    let mut warnings: Vec<String> = Vec::new();
    let verdict = match op.kind {
        OpKind::Execute => validate_execute(&op, cfg, &mut warnings),
        OpKind::Delete => validate_delete(&op, cfg, &mut warnings),
        OpKind::Move | OpKind::Rename => validate_move(&op, cfg, &mut warnings),
        OpKind::EmptyRecycleBin => {
            warnings.push("permanently empties the Windows Recycle Bin — cannot be undone".into());
            Verdict::Approved
        }
    };
    ValidatedOp {
        op,
        tier,
        verdict,
        warnings,
    }
}

fn validate_move(op: &Operation, cfg: &SafetyConfig, warnings: &mut Vec<String>) -> Verdict {
    let src = match check_mutable(Path::new(&op.source), cfg, true) {
        Ok(p) => p,
        Err(e) => return Verdict::Rejected { reason: e },
    };
    let dest_str = match &op.destination {
        Some(d) => d,
        None => {
            return Verdict::Rejected {
                reason: "missing destination".into(),
            }
        }
    };
    let dest = match check_mutable(Path::new(dest_str), cfg, false) {
        Ok(p) => p,
        Err(e) => return Verdict::Rejected { reason: e },
    };
    if paths::paths_equal(&src, &dest) {
        return Verdict::Rejected {
            reason: "source and destination are identical".into(),
        };
    }
    if dest.exists() {
        warnings.push("destination exists — will be auto-renamed to avoid overwrite".into());
    } else if let Some(parent) = dest.parent() {
        if !parent.exists() {
            warnings.push("destination folder will be created".into());
        }
    }
    Verdict::Approved
}

fn validate_delete(op: &Operation, cfg: &SafetyConfig, warnings: &mut Vec<String>) -> Verdict {
    match check_mutable(Path::new(&op.source), cfg, true) {
        Ok(_) => {
            warnings.push("moved to the AI Trash (recoverable)".into());
            Verdict::Approved
        }
        Err(e) => Verdict::Rejected { reason: e },
    }
}

fn validate_execute(op: &Operation, cfg: &SafetyConfig, warnings: &mut Vec<String>) -> Verdict {
    if !cfg.allow_execute {
        return Verdict::Rejected {
            reason: "program execution is disabled (enable in Settings — high risk)".into(),
        };
    }
    let resolved = match paths::resolve(Path::new(&op.source)) {
        Ok(p) => p,
        Err(e) => {
            return Verdict::Rejected {
                reason: format!("cannot resolve program path: {e}"),
            }
        }
    };
    if !resolved.exists() {
        return Verdict::Rejected {
            reason: "program not found".into(),
        };
    }
    for prot in &cfg.protected {
        if paths::path_under(&resolved, prot) {
            return Verdict::Rejected {
                reason: format!("refusing to execute from protected location ({})", prot.display()),
            };
        }
    }
    warnings.push("launches an external program — review carefully".into());
    Verdict::Approved
}

/// Core path gate shared by all mutating operations.
fn check_mutable(p: &Path, cfg: &SafetyConfig, require_exists: bool) -> Result<PathBuf, String> {
    let resolved = paths::resolve(p)
        .map_err(|e| format!("cannot resolve path '{}': {e}", p.display()))?;

    if require_exists && !resolved.exists() {
        return Err(format!("path does not exist: {}", resolved.display()));
    }
    if paths::is_root(&resolved) {
        return Err("refusing to operate on a drive/filesystem root".into());
    }
    for prot in &cfg.protected {
        if paths::path_under(&resolved, prot) {
            return Err(format!("protected system location ({})", prot.display()));
        }
    }
    if paths::path_under(&resolved, &cfg.app_root) {
        return Err("refusing to modify the assistant's own data directory".into());
    }
    let in_sandbox = cfg.managed.iter().any(|m| paths::path_under(&resolved, m));
    if !in_sandbox {
        return Err(
            "outside the allowed folders — add this location in Settings to permit changes".into(),
        );
    }
    Ok(resolved)
}

/// Re-validate an operation's paths at execution time (defense-in-depth against
/// anything that changed between approval and execution, e.g. a symlink/junction
/// swapped in at the destination). Mirrors the checks in `validate_op`.
pub fn recheck_op(op: &Operation, cfg: &SafetyConfig) -> Result<(), String> {
    match op.kind {
        OpKind::Delete => {
            check_mutable(Path::new(&op.source), cfg, true)?;
        }
        OpKind::Move | OpKind::Rename => {
            check_mutable(Path::new(&op.source), cfg, true)?;
            let dest = op
                .destination
                .as_deref()
                .ok_or_else(|| "missing destination".to_string())?;
            check_mutable(Path::new(dest), cfg, false)?;
        }
        OpKind::Execute => {
            if !cfg.allow_execute {
                return Err("program execution is disabled".into());
            }
            let resolved = paths::resolve(Path::new(&op.source))
                .map_err(|e| format!("cannot resolve program path: {e}"))?;
            if !resolved.exists() {
                return Err("program not found".into());
            }
            for prot in &cfg.protected {
                if paths::path_under(&resolved, prot) {
                    return Err(format!(
                        "refusing to execute from protected location ({})",
                        prot.display()
                    ));
                }
            }
        }
        OpKind::EmptyRecycleBin => {}
    }
    Ok(())
}

/// Re-check a single concrete path (used for the auto-renamed move destination).
pub fn check_path(p: &Path, cfg: &SafetyConfig, require_exists: bool) -> Result<(), String> {
    check_mutable(p, cfg, require_exists).map(|_| ())
}

fn collect_location(locs: &mut Vec<String>, path: &str) {
    let p = Path::new(path);
    let dir = p
        .parent()
        .map(|d| d.display().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.to_string());
    if !locs.contains(&dir) {
        locs.push(dir);
    }
}

fn short(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SafetyConfig {
        SafetyConfig {
            protected: vec![PathBuf::from("C:\\Windows"), PathBuf::from("C:\\Program Files")],
            managed: vec![PathBuf::from("C:\\Users\\test")],
            app_root: PathBuf::from("C:\\AI_Assistant"),
            allow_execute: false,
        }
    }

    fn op(kind: OpKind, source: &str, dest: Option<&str>) -> Operation {
        Operation {
            id: "1".into(),
            kind,
            source: source.into(),
            destination: dest.map(|s| s.into()),
            size_bytes: 0,
            is_dir: false,
            file_count: 0,
            args: vec![],
        }
    }

    #[test]
    fn rejects_protected_destination() {
        let p = validate(
            "t".into(),
            vec![op(OpKind::Move, "C:\\Users\\test\\a.txt", Some("C:\\Windows\\a.txt"))],
            &cfg(),
        );
        assert!(!p.has_approved());
    }

    #[test]
    fn rejects_outside_sandbox() {
        let p = validate(
            "t".into(),
            vec![op(OpKind::Delete, "D:\\somewhere\\a.txt", None)],
            &cfg(),
        );
        assert!(!p.has_approved());
    }

    #[test]
    fn execute_disabled_by_default() {
        let p = validate(
            "t".into(),
            vec![op(OpKind::Execute, "C:\\Users\\test\\app.exe", None)],
            &cfg(),
        );
        assert!(!p.has_approved());
    }
}

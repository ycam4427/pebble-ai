//! The Executor. Its public entry point accepts only a `&ValidatedPlan`, so it
//! is impossible to execute anything the Safety Validator did not approve. Each
//! performed operation is written to the action log with the data needed to undo it.

use crate::models::OpKind;
use crate::safety::{SafetyConfig, ValidatedPlan};
use crate::trash::{self, TrashConfig};
use crate::{db, fsutil};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExecutionReport {
    pub executed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub log_ids: Vec<String>,
}

/// Execute every approved operation in a validated plan.
pub fn execute(
    plan: &ValidatedPlan,
    conn: &Connection,
    trash_cfg: &TrashConfig,
    cfg: &SafetyConfig,
) -> ExecutionReport {
    let mut report = ExecutionReport::default();

    for (i, vo) in plan.ops().iter().enumerate() {
        // Rejected/non-approved operations are never executed.
        if !vo.is_approved() {
            report.skipped += 1;
            continue;
        }
        let op = &vo.op;
        let tier = vo.tier.level();
        let idx = i as i64;

        // Defense-in-depth: re-validate the paths NOW, in case something changed
        // between approval and execution (e.g. a symlink/junction swapped in).
        if let Err(reason) = crate::safety::validator::recheck_op(op, cfg) {
            report.failed += 1;
            report
                .errors
                .push(format!("{}: blocked at execution ({reason})", op.source));
            let _ = db::insert_action_log(
                conn,
                Some(plan.id()),
                idx,
                op.kind.as_str(),
                tier,
                &op.source,
                op.destination.as_deref(),
                "failed",
                None,
                Some(&format!("blocked at execution: {reason}")),
            );
            continue;
        }

        let result = match op.kind {
            OpKind::Move | OpKind::Rename => do_move(conn, plan.id(), idx, op, tier, cfg),
            OpKind::Delete => do_delete(conn, plan.id(), idx, op, tier, trash_cfg),
            OpKind::Execute => do_execute(conn, plan.id(), idx, op, tier),
            OpKind::EmptyRecycleBin => do_empty_recycle_bin(conn, plan.id(), idx, op, tier),
        };

        match result {
            Ok(log_id) => {
                report.executed += 1;
                report.log_ids.push(log_id);
            }
            Err(e) => {
                report.failed += 1;
                report.errors.push(format!("{}: {e}", op.source));
                let _ = db::insert_action_log(
                    conn,
                    Some(plan.id()),
                    idx,
                    op.kind.as_str(),
                    tier,
                    &op.source,
                    op.destination.as_deref(),
                    "failed",
                    None,
                    Some(&e.to_string()),
                );
            }
        }
    }
    report
}

fn do_move(
    conn: &Connection,
    plan_id: &str,
    idx: i64,
    op: &crate::models::Operation,
    tier: u8,
    cfg: &SafetyConfig,
) -> Result<String> {
    let src = Path::new(&op.source);
    let dst_str = op
        .destination
        .as_ref()
        .ok_or_else(|| anyhow!("missing destination"))?;
    let dst = fsutil::unique_dest(Path::new(dst_str));
    // The auto-renamed final destination must still pass the safety gate.
    if let Err(reason) = crate::safety::validator::check_path(&dst, cfg, false) {
        return Err(anyhow!("destination blocked at execution: {reason}"));
    }
    fsutil::move_path(src, &dst)?;

    let undo = json!({
        "type": "move",
        "from": dst.display().to_string(),
        "to": op.source,
    })
    .to_string();

    let id = db::insert_action_log(
        conn,
        Some(plan_id),
        idx,
        op.kind.as_str(),
        tier,
        &op.source,
        Some(&dst.display().to_string()),
        "executed",
        Some(&undo),
        None,
    )?;
    Ok(id)
}

fn do_delete(
    conn: &Connection,
    plan_id: &str,
    idx: i64,
    op: &crate::models::Operation,
    tier: u8,
    trash_cfg: &TrashConfig,
) -> Result<String> {
    let item = trash::move_to_trash(conn, Path::new(&op.source), trash_cfg)?;
    let undo = json!({ "type": "delete", "trash_id": item.id }).to_string();
    let id = db::insert_action_log(
        conn,
        Some(plan_id),
        idx,
        "delete",
        tier,
        &op.source,
        Some(&item.trash_path),
        "executed",
        Some(&undo),
        None,
    )?;
    Ok(id)
}

fn do_empty_recycle_bin(
    conn: &Connection,
    plan_id: &str,
    idx: i64,
    op: &crate::models::Operation,
    tier: u8,
) -> Result<String> {
    #[cfg(windows)]
    {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Clear-RecycleBin -Force -Confirm:$false"])
            .output()
            .map_err(|e| anyhow!("couldn't run Clear-RecycleBin: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).to_lowercase();
            // "The Recycle Bin is empty." is not a real error.
            if !err.contains("empty") {
                return Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim()));
            }
        }
    }
    #[cfg(not(windows))]
    {
        return Err(anyhow!("emptying the Recycle Bin is only supported on Windows"));
    }
    let id = db::insert_action_log(
        conn,
        Some(plan_id),
        idx,
        "empty_recycle_bin",
        tier,
        &op.source,
        None,
        "executed",
        None,
        None,
    )?;
    Ok(id)
}

fn do_execute(
    conn: &Connection,
    plan_id: &str,
    idx: i64,
    op: &crate::models::Operation,
    tier: u8,
) -> Result<String> {
    // Validator already gated this behind the allow_execute setting + high-risk
    // confirmation; we launch detached and do not wait.
    Command::new(&op.source)
        .args(&op.args)
        .spawn()
        .map_err(|e| anyhow!("failed to launch: {e}"))?;
    let id = db::insert_action_log(
        conn,
        Some(plan_id),
        idx,
        "execute",
        tier,
        &op.source,
        None,
        "executed",
        None,
        None,
    )?;
    Ok(id)
}

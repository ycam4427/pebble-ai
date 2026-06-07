import { useState } from "react";
import { AlertTriangle, ArrowRight, Check, ChevronDown, ShieldAlert, X } from "lucide-react";
import type { ValidatedOp, ValidatedPlan } from "../../lib/types";
import { useStore } from "../../store/appStore";
import { bytes, fileName, tierLabel } from "../../lib/format";
import Pebble from "../Pebble";

const KIND_LABEL: Record<string, string> = {
  move: "Move",
  rename: "Rename",
  delete: "Delete",
  execute: "Run",
  empty_recycle_bin: "Empty Bin",
};

function shorten(p: string) {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts.length <= 2 ? p : "…\\" + parts.slice(-2).join("\\");
}

function Count({ n, l }: { n: number; l: string }) {
  return (
    <div className="count-box">
      <div className="n">{n}</div>
      <div className="l">{l}</div>
    </div>
  );
}

function OpRow({ vo }: { vo: ValidatedOp }) {
  const rejected = vo.verdict.status === "rejected";
  const o = vo.op;
  return (
    <div className={`op ${rejected ? "rejected" : ""}`}>
      <span className="op-kind">{KIND_LABEL[o.kind] ?? o.kind}</span>
      <div className="op-detail">
        <div className="a" title={o.source}>
          {fileName(o.source)}
          {o.is_dir ? "/" : ""} {o.size_bytes > 0 && <span className="faint">· {bytes(o.size_bytes)}</span>}
        </div>
        {o.destination && (
          <div className="b" title={o.destination}>
            <ArrowRight size={10} /> {o.destination}
          </div>
        )}
        {vo.verdict.status === "rejected" && <div className="reason">{vo.verdict.reason}</div>}
      </div>
    </div>
  );
}

export default function PlanView({ plan }: { plan: ValidatedPlan }) {
  const approve = useStore((s) => s.approve);
  const reject = useStore((s) => s.reject);
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [showOps, setShowOps] = useState(false);

  const approvedOps = plan.ops.filter((o) => o.verdict.status === "approved");
  const hasApproved = approvedOps.length > 0;
  const needsTyped = plan.requires_typed_confirmation;
  const phrase = plan.confirmation_phrase ?? "";
  const canApprove = hasApproved && (!needsTyped || typed.trim() === phrase);
  const totalCounts = plan.move_count + plan.rename_count + plan.delete_count + plan.execute_count;

  const doApprove = async () => {
    setBusy(true);
    await approve(plan.id, needsTyped ? typed : undefined);
    setBusy(false);
  };
  const doReject = async () => {
    setBusy(true);
    await reject(plan.id);
    setBusy(false);
  };

  return (
    <div className="plan">
      <div className="plan-top">
        <Pebble size={26} className="peb" />
        <div className="plan-summary">{plan.summary}</div>
        <span className={`tier-tag tier-${plan.max_tier}`}>{tierLabel(plan.max_tier)}</span>
      </div>

      <div className="op-counts">
        {plan.move_count > 0 && <Count n={plan.move_count} l="Move" />}
        {plan.rename_count > 0 && <Count n={plan.rename_count} l="Rename" />}
        {plan.delete_count > 0 && <Count n={plan.delete_count} l="Delete" />}
        {plan.execute_count > 0 && <Count n={plan.execute_count} l="Run" />}
        {totalCounts === 0 && <Count n={0} l="Actions" />}
      </div>

      {plan.affected_locations.length > 0 && (
        <div>
          <div className="section-label">Affected locations</div>
          <div className="locations">
            {plan.affected_locations.map((l, i) => (
              <span className="loc-pill" key={i} title={l}>
                {shorten(l)}
              </span>
            ))}
          </div>
        </div>
      )}

      {(plan.warnings.length > 0 || plan.rejected.length > 0) && (
        <div className="warnings">
          {plan.warnings.map((w, i) => (
            <div className="warn" key={i}>
              <AlertTriangle size={14} />
              {w}
            </div>
          ))}
          {plan.rejected.map((r, i) => (
            <div className="warn blocked" key={"r" + i}>
              <ShieldAlert size={14} />
              Blocked: {r}
            </div>
          ))}
        </div>
      )}

      {plan.ops.length > 0 && (
        <div>
          <button className="disclosure" onClick={() => setShowOps((o) => !o)}>
            <ChevronDown
              size={14}
              style={{ transform: showOps ? "rotate(180deg)" : "none", transition: "transform .2s" }}
            />
            {showOps ? "Hide" : "Show"} the details ({plan.ops.length})
          </button>
          {showOps && (
            <div className="op-list" style={{ marginTop: 8 }}>
              {plan.ops.map((vo, i) => (
                <OpRow key={i} vo={vo} />
              ))}
            </div>
          )}
        </div>
      )}

      {needsTyped && (
        <div className="highrisk">
          <h4>
            <AlertTriangle size={16} /> Just to be safe
          </h4>
          <div className="muted" style={{ fontSize: 12.5 }}>
            This is a big one. Type <b>{phrase}</b> exactly and I'll take care of it.
          </div>
          <input
            className="confirm-input"
            value={typed}
            placeholder={phrase}
            onChange={(e) => setTyped(e.target.value)}
          />
        </div>
      )}

      <div className="actions-row">
        <button className="btn ghost" onClick={doReject} disabled={busy}>
          <X size={15} /> Maybe later
        </button>
        {hasApproved ? (
          <button
            className={`btn ${plan.max_tier === "high_risk" ? "danger" : "primary"}`}
            onClick={doApprove}
            disabled={!canApprove || busy}
          >
            <Check size={15} /> Yes please{plan.max_tier !== "auto" ? ` (${approvedOps.length})` : ""}
          </button>
        ) : (
          <button className="btn" disabled style={{ flex: 1 }}>
            Nothing to approve
          </button>
        )}
      </div>
    </div>
  );
}

import { useEffect } from "react";
import { ArrowRight, Undo2 } from "lucide-react";
import { useStore } from "../../store/appStore";
import { dateTime, fileName } from "../../lib/format";
import Pebble from "../Pebble";

const UNDOABLE = new Set(["move", "rename", "delete"]);

export default function HistoryView() {
  const { history, refreshHistory, undoLast, undoOne } = useStore();

  useEffect(() => {
    refreshHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const canUndoAny = history.some((h) => h.status === "executed" && UNDOABLE.has(h.kind));

  return (
    <div>
      <div className="toolbar">
        <div className="breadcrumb">
          <b>{history.length}</b>&nbsp;logged action(s)
        </div>
        <button className="btn sm" onClick={() => undoLast()} disabled={!canUndoAny}>
          <Undo2 size={14} /> Undo last
        </button>
      </div>

      {history.length === 0 && (
        <div className="empty">
          <Pebble size={60} />
          <div style={{ marginTop: 6 }}>No actions yet — everything Pebble does shows up here.</div>
        </div>
      )}

      {history.map((h) => (
        <div className="data-card" key={h.id}>
          <span className="op-kind">{h.kind}</span>
          <div className="grow">
            <div className="t" title={h.source}>
              {fileName(h.source)}
              {h.destination && (
                <>
                  {" "}
                  <ArrowRight size={11} style={{ verticalAlign: "-1px" }} /> {fileName(h.destination)}
                </>
              )}
            </div>
            <div className="faint" style={{ fontSize: 11 }}>
              {dateTime(h.executed_at)}
              {h.error ? ` · ${h.error}` : ""}
            </div>
          </div>
          <span className={`status-pill status-${h.status}`}>{h.status}</span>
          {h.status === "executed" && UNDOABLE.has(h.kind) && (
            <button className="btn sm" onClick={() => undoOne(h.id)}>
              <Undo2 size={13} /> Undo
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

import { useEffect } from "react";
import { FolderInput, RotateCcw, Trash2 } from "lucide-react";
import { useStore } from "../../store/appStore";
import { bytes, dateTime, timeAgo } from "../../lib/format";
import Pebble from "../Pebble";

export default function TrashView() {
  const { trash, refreshTrash, restore, removeTrashItem, emptyTrash, config } = useStore();

  useEffect(() => {
    refreshTrash();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div>
      <div className="toolbar">
        <div className="breadcrumb">
          <b>{trash.length}</b>&nbsp;item(s) in Trash
        </div>
        <span className="faint" style={{ fontSize: 12 }}>
          Auto-cleanup after {config?.retention_days ?? 30} days · {config?.trash_root}
        </span>
        <button className="btn sm danger" disabled={trash.length === 0} onClick={() => emptyTrash()}>
          <Trash2 size={14} /> Empty Trash
        </button>
      </div>

      {trash.length === 0 && (
        <div className="empty">
          <Pebble size={60} />
          <div style={{ marginTop: 6 }}>Nothing in the Trash — squeaky clean 🤍</div>
        </div>
      )}

      {trash.map((t) => (
        <div className="data-card" key={t.id}>
          <FolderInput size={18} color="var(--text-faint)" />
          <div className="grow">
            <div className="t">{t.name}</div>
            <div className="s" title={t.original_path}>
              from {t.original_path}
            </div>
            <div className="faint" style={{ fontSize: 11 }}>
              {bytes(t.size)} · deleted {timeAgo(t.deleted_at)} · expires {dateTime(t.expires_at)}
            </div>
          </div>
          <button className="btn sm" onClick={() => restore(t.id)}>
            <RotateCcw size={14} /> Restore
          </button>
          <button
            className="btn sm ghost"
            onClick={() => removeTrashItem(t.id)}
            title="Delete permanently"
          >
            <Trash2 size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}

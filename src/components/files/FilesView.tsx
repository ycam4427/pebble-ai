import { useEffect, useState } from "react";
import { ArrowUp, BarChart3, ChevronDown, File, Folder, RefreshCw, Trash2 } from "lucide-react";
import { useStore } from "../../store/appStore";
import * as api from "../../lib/ipc";
import type { FileEntry, FolderAnalysis } from "../../lib/types";
import { bytes, timeAgo } from "../../lib/format";
import { Dropdown } from "../Dropdown";
import Pebble from "../Pebble";

export default function FilesView() {
  const config = useStore((s) => s.config);
  const proposeTrash = useStore((s) => s.proposeTrash);
  const lastReport = useStore((s) => s.lastReport);

  const [path, setPath] = useState("");
  const [items, setItems] = useState<FileEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [analysis, setAnalysis] = useState<FolderAnalysis | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!path && config) {
      const home =
        config.known_folders.find((k) => k.label === "Home")?.path ??
        config.known_folders[0]?.path ??
        "";
      if (home) setPath(home);
    }
  }, [config, path]);

  const load = async (p: string) => {
    setLoading(true);
    setSelected(new Set());
    setAnalysis(null);
    try {
      setItems(await api.fsListDir(p));
    } catch {
      setItems([]);
    }
    setLoading(false);
  };

  useEffect(() => {
    if (path) load(path);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  useEffect(() => {
    if (path && lastReport) load(path);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lastReport]);

  const crumbs = path.split(/[\\/]/).filter(Boolean);
  const isWin = /^[A-Za-z]:$/.test(crumbs[0] || "");
  const goCrumb = (i: number) => {
    const parts = crumbs.slice(0, i + 1);
    let np = isWin ? parts.join("\\") : "/" + parts.join("/");
    if (isWin && parts.length === 1) np = parts[0] + "\\";
    setPath(np);
  };
  const up = () => {
    if (crumbs.length > 1) goCrumb(crumbs.length - 2);
  };

  const toggle = (p: string) => {
    const s = new Set(selected);
    if (s.has(p)) s.delete(p);
    else s.add(p);
    setSelected(s);
  };

  const analyze = async () => {
    if (!path) return;
    setLoading(true);
    try {
      setAnalysis(await api.fsAnalyze(path));
    } catch {
      /* ignore */
    }
    setLoading(false);
  };

  return (
    <div>
      <div className="toolbar">
        <button className="menu-btn" onClick={up} title="Up one level">
          <ArrowUp size={15} />
        </button>
        <div className="breadcrumb">
          {crumbs.map((c, i) => (
            <span key={i}>
              <button className="crumb" onClick={() => goCrumb(i)}>
                {c || "/"}
              </button>
              {i < crumbs.length - 1 && <span className="faint">›</span>}
            </span>
          ))}
        </div>
        {config && (
          <Dropdown
            trigger={
              <>
                Jump to <ChevronDown size={14} />
              </>
            }
          >
            {(close) =>
              config.known_folders.map((k) => (
                <button
                  key={k.path}
                  className="menu-item"
                  onClick={() => {
                    setPath(k.path);
                    close();
                  }}
                >
                  <Folder size={15} /> {k.label}
                </button>
              ))
            }
          </Dropdown>
        )}
        <button className="btn sm" onClick={() => load(path)}>
          <RefreshCw size={14} /> Refresh
        </button>
        <button className="btn sm" onClick={analyze}>
          <BarChart3 size={14} /> Analyze
        </button>
        <button
          className="btn sm danger"
          disabled={selected.size === 0}
          onClick={() => proposeTrash([...selected])}
        >
          <Trash2 size={14} /> Trash{selected.size ? ` (${selected.size})` : ""}
        </button>
      </div>

      {analysis && (
        <div className="data-card" style={{ flexDirection: "column", alignItems: "stretch" }}>
          <b>
            {bytes(analysis.total_bytes)} · {analysis.file_count} files · {analysis.dir_count} folders
          </b>
          <div className="bars" style={{ padding: "8px 0 0" }}>
            {analysis.by_category.map((s) => (
              <div className="bar-row" key={s.category}>
                <span>{s.category}</span>
                <div className="bar-track">
                  <div
                    className="bar-fill"
                    style={{ width: `${Math.max(2, (s.bytes / Math.max(analysis.total_bytes, 1)) * 100)}%` }}
                  />
                </div>
                <span className="fsize">{bytes(s.bytes)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="list">
        {loading && <div className="faint" style={{ padding: 12 }}>Loading…</div>}
        {!loading && items.length === 0 && (
          <div className="empty">
            <Pebble size={56} />
            <div>This folder is empty.</div>
          </div>
        )}
        {items.map((f) => (
          <div className={`row ${selected.has(f.path) ? "selected" : ""}`} key={f.path}>
            <input
              type="checkbox"
              className="checkbox"
              checked={selected.has(f.path)}
              onChange={() => toggle(f.path)}
            />
            <div
              className="name"
              style={{ cursor: "pointer" }}
              onClick={() => (f.is_dir ? setPath(f.path) : toggle(f.path))}
            >
              {f.is_dir ? <Folder size={16} color="var(--accent-strong)" /> : <File size={16} color="var(--text-faint)" />}
              <span>{f.name}</span>
            </div>
            <span className="meta">{f.is_dir ? "" : bytes(f.size)}</span>
            <span className="meta">{timeAgo(f.modified)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

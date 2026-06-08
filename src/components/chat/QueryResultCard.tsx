import { type ReactNode } from "react";
import {
  AlertTriangle,
  BookOpen,
  Clock,
  Cloud,
  FileText,
  Files,
  FolderTree,
  Globe,
  HardDrive,
  Search,
} from "lucide-react";
import type { CategoryStat, FileEntry, QueryResult } from "../../lib/types";
import { bytes, fileName } from "../../lib/format";
import * as api from "../../lib/ipc";

function Card({
  icon,
  title,
  sub,
  children,
}: {
  icon: ReactNode;
  title: string;
  sub?: string;
  children: ReactNode;
}) {
  return (
    <div className="card">
      <div className="card-head">
        {icon}
        {title}
        {sub && <span className="sub">{sub}</span>}
      </div>
      <div className="card-body">{children}</div>
    </div>
  );
}

function FileRows({ files }: { files: FileEntry[] }) {
  if (files.length === 0) return <div className="faint" style={{ padding: "10px 14px" }}>Nothing found.</div>;
  return (
    <div className="filelist">
      {files.map((f, i) => (
        <div className="file-row" key={i} title={f.path}>
          <span className="fname">{f.name || fileName(f.path)}</span>
          <span className="fsize">{f.is_dir ? "folder" : bytes(f.size)}</span>
        </div>
      ))}
    </div>
  );
}

function Bars({ stats, total }: { stats: CategoryStat[]; total: number }) {
  const max = Math.max(total, 1);
  return (
    <div className="bars">
      {stats.map((s) => (
        <div className="bar-row" key={s.category}>
          <span>{s.category}</span>
          <div className="bar-track">
            <div className="bar-fill" style={{ width: `${Math.max(2, (s.bytes / max) * 100)}%` }} />
          </div>
          <span className="fsize">{bytes(s.bytes)}</span>
        </div>
      ))}
    </div>
  );
}

export default function QueryResultCard({ result }: { result: QueryResult }) {
  switch (result.type) {
    case "large_files":
      return (
        <Card icon={<Files size={15} />} title="Largest files" sub={`${result.files.length} found`}>
          <FileRows files={result.files} />
        </Card>
      );
    case "stale_files":
      return (
        <Card
          icon={<Clock size={15} />}
          title={`Not opened in ${result.days}+ days`}
          sub={`${result.files.length} files`}
        >
          <FileRows files={result.files} />
        </Card>
      );
    case "search_results":
      return (
        <Card
          icon={<Search size={15} />}
          title={`Results for "${result.query}"`}
          sub={`${result.files.length} matches`}
        >
          <FileRows files={result.files} />
        </Card>
      );
    case "content_matches":
      return (
        <Card
          icon={<Search size={15} />}
          title={`Inside files · "${result.query}"`}
          sub={`${result.matches.length} match${result.matches.length === 1 ? "" : "es"}`}
        >
          {result.matches.length === 0 && (
            <div className="faint" style={{ padding: "10px 14px" }}>
              No files contained that.
            </div>
          )}
          {result.matches.map((m, i) => (
            <div className="file-row" key={i} title={m.path} style={{ display: "block" }}>
              <div className="fname">
                {m.name || fileName(m.path)} <span className="faint">· line {m.line}</span>
              </div>
              <div
                className="faint"
                style={{ fontSize: 12, whiteSpace: "pre-wrap", wordBreak: "break-word" }}
              >
                {m.snippet}
              </div>
            </div>
          ))}
        </Card>
      );
    case "duplicates": {
      const wasted = result.groups.reduce((a, g) => a + g.size * (g.files.length - 1), 0);
      return (
        <Card
          icon={<Files size={15} />}
          title="Duplicate files"
          sub={`${result.groups.length} groups · ${bytes(wasted)} reclaimable`}
        >
          {result.groups.slice(0, 40).map((g, i) => (
            <div className="dup-group" key={i}>
              <div className="muted" style={{ fontSize: 12, marginBottom: 4 }}>
                {g.files.length} copies · {bytes(g.size)} each
              </div>
              {g.files.map((f, j) => (
                <div className="file-row" key={j} title={f.path}>
                  <span className="fname">{f.path}</span>
                </div>
              ))}
            </div>
          ))}
        </Card>
      );
    }
    case "storage": {
      const s = result.stats;
      return (
        <Card
          icon={<HardDrive size={15} />}
          title="Storage usage"
          sub={`${bytes(s.total_bytes)} · ${s.file_count} files`}
        >
          <Bars stats={s.by_category} total={s.total_bytes} />
          {s.largest.length > 0 && (
            <>
              <div className="section-label" style={{ padding: "4px 14px" }}>
                Largest
              </div>
              <FileRows files={s.largest} />
            </>
          )}
        </Card>
      );
    }
    case "folder_analysis": {
      const a = result.analysis;
      return (
        <Card
          icon={<FolderTree size={15} />}
          title="Folder analysis"
          sub={`${bytes(a.total_bytes)} · ${a.file_count} files · ${a.dir_count} folders`}
        >
          <Bars stats={a.by_category} total={a.total_bytes} />
        </Card>
      );
    }
    case "file_content":
      return (
        <Card
          icon={<FileText size={15} />}
          title={fileName(result.path)}
          sub={result.truncated ? "preview (truncated)" : "preview"}
        >
          <pre className="preview">{result.preview}</pre>
        </Card>
      );
    case "summary":
      return (
        <Card icon={<BookOpen size={15} />} title={`Summary · ${fileName(result.path)}`}>
          <div style={{ padding: "10px 14px" }}>{result.summary}</div>
        </Card>
      );
    case "web_results":
      return (
        <Card
          icon={<Globe size={15} />}
          title={`Web results for "${result.query}"`}
          sub={`${result.results.length} found`}
        >
          {result.results.length === 0 && (
            <div className="faint" style={{ padding: "10px 14px" }}>
              No results.
            </div>
          )}
          {result.results.map((r, i) => (
            <div className="web-row" key={i}>
              <button className="web-title" onClick={() => api.openUrl(r.url)} title={r.url}>
                {r.title}
              </button>
              <div className="web-url">{r.url}</div>
              {r.snippet && <div className="web-snippet">{r.snippet}</div>}
            </div>
          ))}
        </Card>
      );
    case "weather": {
      const w = result.info;
      return (
        <Card icon={<Cloud size={15} />} title={`Weather · ${w.location}`} sub={w.description}>
          <div style={{ padding: "10px 14px", display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ fontSize: 22, fontWeight: 700 }}>
              {w.temp_c}°C{" "}
              <span className="faint" style={{ fontSize: 13, fontWeight: 400 }}>
                ({w.temp_f}°F)
              </span>
            </div>
            <div className="faint" style={{ fontSize: 13 }}>
              Feels like {w.feels_like_c}°C · Humidity {w.humidity}% · Wind {w.wind_kmph} km/h
            </div>
          </div>
        </Card>
      );
    }
    case "error":
      return (
        <Card icon={<AlertTriangle size={15} />} title="Couldn't complete that">
          <div style={{ padding: "10px 14px", color: "#ffb4b4" }}>{result.message}</div>
        </Card>
      );
  }
}

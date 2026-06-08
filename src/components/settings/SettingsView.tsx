import { useEffect, useState } from "react";
import {
  Brain,
  Cloud,
  Cpu,
  Database,
  FolderLock,
  HardDrive,
  Heart,
  MessageCircle,
  Palette,
  Plus,
  Puzzle,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useStore } from "../../store/appStore";
import * as api from "../../lib/ipc";
import { bytes } from "../../lib/format";
import { PERSONAS, THEMES } from "../../lib/themes";
import { blurbFor, DISCORD_URL, MODEL_BLURBS } from "../../lib/constants";
import { Accordion } from "../Accordion";

const SWATCH: Record<string, string[]> = {
  pebble: ["#fbf6f8", "#e7a6bd", "#efe2ea"],
  matcha: ["#fffef8", "#a9c994", "#e2e7d2"],
  cloud: ["#ffffff", "#b7c0ea", "#e6e8ef"],
  stars: ["#0a0f2b", "#bcacff", "#2a3266"],
  liquidglass: ["#081530", "#7cc7ff", "#103a6b"],
};

function ExtToggle({
  label,
  desc,
  on,
  onToggle,
}: {
  label: string;
  desc: string;
  on: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="toggle" style={{ marginBottom: 14 }}>
      <div>
        <div>{label}</div>
        <div className="faint" style={{ fontSize: 12 }}>
          {desc}
        </div>
      </div>
      <button className={`switch ${on ? "on" : ""}`} onClick={onToggle} aria-label={`Toggle ${label}`}>
        <span className="knob" />
      </button>
    </div>
  );
}

export default function SettingsView() {
  const {
    config,
    models,
    refreshModels,
    saveSettings,
    setTheme,
    ollama,
    plugins,
    pulling,
    pullModel,
    system,
    memories,
    refreshMemory,
    deleteMemoryItem,
    clearAllMemory,
  } = useStore();
  const [url, setUrl] = useState("");
  const [retention, setRetention] = useState(30);
  const [newRoot, setNewRoot] = useState("");
  const [name, setName] = useState("");
  const [about, setAbout] = useState("");

  useEffect(() => {
    refreshModels();
    refreshMemory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (config) {
      setUrl(config.ollama_url);
      setRetention(config.retention_days);
      setName(config.user_name);
      setAbout(config.about_you);
    }
  }, [config]);

  if (!config) return <div className="empty">Loading…</div>;

  const selectedBlurb = blurbFor(config.model);
  const recInstalled = system ? models.some((m) => m.name === system.recommended_model) : false;

  return (
    <div className="settings-wrap">
      {/* You & Pebble ----------------------------------------------------- */}
      <Accordion icon={<Heart size={17} />} title="You & Pebble" sub="how Pebble talks to you" defaultOpen>
        <div className="field">
          <label>What should Pebble call you?</label>
          <div className="inline">
            <input
              type="text"
              value={name}
              maxLength={40}
              placeholder="your name"
              onChange={(e) => setName(e.target.value)}
              style={{ flex: 1 }}
            />
            <button className="btn sm" onClick={() => saveSettings({ user_name: name.trim() })}>
              Save
            </button>
          </div>
        </div>

        <div className="field">
          <label>Pebble's vibe</label>
          <div className="choice-grid">
            {PERSONAS.map((p) => (
              <button
                key={p.key}
                className={`choice ${config.persona === p.key ? "active" : ""}`}
                onClick={() => saveSettings({ persona: p.key })}
              >
                <div className="ct">
                  {p.emoji} {p.label}
                </div>
                <div className="cd">{p.desc}</div>
              </button>
            ))}
          </div>
        </div>

        <div className="field">
          <label>Anything Pebble should know about you? (optional)</label>
          <textarea
            value={about}
            maxLength={400}
            placeholder="e.g. I'm a student, I keep lots of screenshots, please be extra gentle…"
            onChange={(e) => setAbout(e.target.value)}
          />
          <div className="inline" style={{ justifyContent: "flex-end" }}>
            <button className="btn sm" onClick={() => saveSettings({ about_you: about.trim() })}>
              Save
            </button>
          </div>
        </div>

        <div className="toggle" style={{ marginTop: 4 }}>
          <div>
            <div>Match my tone</div>
            <div className="faint" style={{ fontSize: 12 }}>
              When on, Pebble mirrors your energy and length — playful when you're playful, gentle when
              you're down, brief when you're brief.
            </div>
          </div>
          <button
            className={`switch ${config.adapt_tone ? "on" : ""}`}
            onClick={() => saveSettings({ adapt_tone: !config.adapt_tone })}
            aria-label="Toggle tone matching"
          >
            <span className="knob" />
          </button>
        </div>
      </Accordion>

      {/* Pebble's Memory ------------------------------------------------- */}
      <Accordion
        icon={<Brain size={17} />}
        title="Pebble's Memory"
        sub={config.allow_memory ? `${memories.length} remembered` : "off"}
      >
        <div className="toggle" style={{ marginBottom: 14 }}>
          <div>
            <div>Let Pebble remember things about you</div>
            <div className="faint" style={{ fontSize: 12 }}>
              Off by default. When on, Pebble quietly keeps a few personal notes (and dates) so he can
              follow up later — like asking how a test went. It all stays on this PC.
            </div>
          </div>
          <button
            className={`switch ${config.allow_memory ? "on" : ""}`}
            onClick={() => saveSettings({ allow_memory: !config.allow_memory })}
            aria-label="Toggle memory"
          >
            <span className="knob" />
          </button>
        </div>

        {config.allow_memory && memories.length === 0 && (
          <div className="faint" style={{ fontSize: 12 }}>
            Nothing yet — chat with Pebble and he'll remember what matters 🤍
          </div>
        )}
        {config.allow_memory && memories.length > 0 && (
          <>
            <div className="roots-list">
              {memories.map((m) => (
                <div className="root-pill" key={m.id} style={{ alignItems: "flex-start", gap: 8 }}>
                  <span style={{ flex: 1 }}>
                    {m.content}
                    {m.event_date && (
                      <span className="faint" style={{ fontSize: 11, display: "block" }}>
                        📅 {m.event_date}
                      </span>
                    )}
                  </span>
                  <button
                    className="icon-btn"
                    title="Forget this"
                    style={{ background: "none", border: "none", color: "inherit" }}
                    onClick={() => deleteMemoryItem(m.id)}
                  >
                    <X size={13} />
                  </button>
                </div>
              ))}
            </div>
            <div className="inline" style={{ justifyContent: "flex-end", marginTop: 8 }}>
              <button className="btn sm" onClick={() => clearAllMemory()}>
                Clear all
              </button>
            </div>
          </>
        )}
      </Accordion>

      {/* Appearance ------------------------------------------------------- */}
      <Accordion icon={<Palette size={17} />} title="Appearance" sub={`theme · ${config.theme}`}>
        <div className="choice-grid">
          {THEMES.map((t) => (
            <button
              key={t.key}
              className={`choice ${config.theme === t.key ? "active" : ""}`}
              onClick={() => setTheme(t.key)}
            >
              <div className="ct">
                {t.label}
                <span className="swatch">
                  {(SWATCH[t.key] ?? []).map((c, i) => (
                    <i key={i} style={{ background: c }} />
                  ))}
                </span>
              </div>
              <div className="cd">{t.desc}</div>
            </button>
          ))}
        </div>
      </Accordion>

      {/* AI model --------------------------------------------------------- */}
      <Accordion icon={<Cpu size={17} />} title="AI Model" sub={config.model}>
        {system && (
          <div className="pc-box">
            <div className="pc-line">
              <HardDrive size={15} />
              {system.gpu_name || "GPU not detected"}
              {system.vram_gb > 0 ? ` · ${system.vram_gb} GB VRAM` : ""} · {system.ram_gb} GB RAM ·{" "}
              {system.cpu_cores} cores
            </div>
            <div className="hint" style={{ margin: "6px 0 0" }}>
              {system.reason}
            </div>
            {system.recommended_model && system.recommended_model !== config.model && (
              <div className="pc-rec">
                <span>
                  Recommended: <b>{system.recommended_model}</b>
                </span>
                {recInstalled ? (
                  <button className="btn sm primary" onClick={() => saveSettings({ model: system.recommended_model })}>
                    Use it
                  </button>
                ) : (
                  <button
                    className="btn sm primary"
                    disabled={!!pulling}
                    onClick={() => pullModel(system.recommended_model)}
                  >
                    {pulling === system.recommended_model ? "Pulling…" : "Pull it"}
                  </button>
                )}
              </div>
            )}
          </div>
        )}

        <div className="hint">
          Runs entirely locally via Ollama.{" "}
          {ollama?.running ? `Connected · v${ollama.version}` : "Ollama is not reachable."} You can switch
          the model anytime.
        </div>
        {models.length === 0 && (
          <div className="muted" style={{ marginBottom: 10 }}>
            No models found. Pull one, e.g. <span className="mono">ollama pull llama3.2</span>
          </div>
        )}
        {models.map((m) => (
          <div
            key={m.name}
            className={`model-row ${m.name === config.model ? "active" : ""}`}
            style={{ cursor: "pointer" }}
            onClick={() => saveSettings({ model: m.name })}
          >
            <Cpu size={16} />
            <div style={{ flex: 1 }}>
              <div className="mname">
                {m.name}
                {m.name === config.model && <span className="faint"> · selected</span>}
              </div>
              <div className="mmeta">
                {m.parameter_size || "?"} · {m.quantization || "?"} · ~{bytes(m.vram_bytes)}{" "}
                {m.loaded ? "VRAM (loaded)" : "est."}
              </div>
            </div>
          </div>
        ))}
        {selectedBlurb && (
          <div className="blurb">
            <b>{selectedBlurb.tier}.</b> ✓ {selectedBlurb.good}{" "}
            <span className="faint">✕ {selectedBlurb.limit}</span>
          </div>
        )}

        <div className="section-label" style={{ marginTop: 16 }}>
          Want a smarter Pebble?
        </div>
        <div className="hint" style={{ marginBottom: 8 }}>
          Bigger models are smarter but need a stronger PC. Pull one, then pick it — pulling downloads
          via Ollama and can take a while.
        </div>
        {MODEL_BLURBS.map((r) => {
          const installed = models.some((m) => m.name === r.name);
          const isPulling = pulling === r.name;
          return (
            <div className="model-row" key={r.name}>
              <Cpu size={16} />
              <div style={{ flex: 1 }}>
                <div className="mname">
                  {r.name} <span className="faint">· {r.tier}</span>
                </div>
                <div className="mmeta">✓ {r.good}</div>
                <div className="mmeta faint">✕ {r.limit}</div>
              </div>
              {installed ? (
                config.model === r.name ? (
                  <span className="faint" style={{ fontSize: 12 }}>
                    selected
                  </span>
                ) : (
                  <button className="btn sm" onClick={() => saveSettings({ model: r.name })}>
                    Use
                  </button>
                )
              ) : (
                <button className="btn sm" disabled={!!pulling} onClick={() => pullModel(r.name)}>
                  {isPulling ? "Pulling…" : "Pull"}
                </button>
              )}
            </div>
          );
        })}

        <div className="field" style={{ marginTop: 12 }}>
          <label>Ollama URL</label>
          <div className="inline">
            <input type="text" value={url} onChange={(e) => setUrl(e.target.value)} style={{ flex: 1 }} />
            <button className="btn sm" onClick={() => saveSettings({ ollama_url: url })}>
              Save
            </button>
          </div>
        </div>
      </Accordion>

      {/* Trash ------------------------------------------------------------ */}
      <Accordion icon={<Trash2 size={17} />} title="Trash & Retention" sub={`${config.retention_days} days`}>
        <div className="hint">Deleted files go to the AI Trash and stay recoverable until cleanup.</div>
        <div className="field">
          <label>Auto-cleanup after (days)</label>
          <div className="inline">
            <input
              type="number"
              min={1}
              value={retention}
              onChange={(e) => setRetention(Number(e.target.value))}
              style={{ width: 120 }}
            />
            <button className="btn sm" onClick={() => saveSettings({ retention_days: retention })}>
              Save
            </button>
          </div>
        </div>
        <div className="root-pill">
          <FolderLock size={14} />
          {config.trash_root}
        </div>
      </Accordion>

      {/* Safety ----------------------------------------------------------- */}
      <Accordion icon={<ShieldCheck size={17} />} title="Safety" sub="protected & managed folders">
        <div className="hint">
          The Safety Validator independently checks every action before it runs. Pebble can only ever
          propose — you approve.
        </div>

        <div className="toggle" style={{ marginBottom: 18 }}>
          <div>
            <div>Allow program execution</div>
            <div className="faint" style={{ fontSize: 12 }}>
              High risk — off by default. Lets Pebble launch programs (still requires confirmation).
            </div>
          </div>
          <button
            className={`switch ${config.allow_execute ? "on" : ""}`}
            onClick={() => saveSettings({ allow_execute: !config.allow_execute })}
            aria-label="Toggle program execution"
          >
            <span className="knob" />
          </button>
        </div>

        <div className="toggle" style={{ marginBottom: 18 }}>
          <div>
            <div>Let Pebble search the web</div>
            <div className="faint" style={{ fontSize: 12 }}>
              Off by default. When on, your search text is sent to DuckDuckGo (this leaves your PC).
            </div>
          </div>
          <button
            className={`switch ${config.allow_web ? "on" : ""}`}
            onClick={() => saveSettings({ allow_web: !config.allow_web })}
            aria-label="Toggle web search"
          >
            <span className="knob" />
          </button>
        </div>

        <div className="toggle" style={{ marginBottom: 18 }}>
          <div>
            <div>
              <Cloud size={13} style={{ verticalAlign: "-2px", marginRight: 6 }} />
              Let Pebble check the weather
            </div>
            <div className="faint" style={{ fontSize: 12 }}>
              Off by default. When on, Pebble can fetch the current weather (this uses the internet).
            </div>
          </div>
          <button
            className={`switch ${config.allow_weather ? "on" : ""}`}
            onClick={() => saveSettings({ allow_weather: !config.allow_weather })}
            aria-label="Toggle weather"
          >
            <span className="knob" />
          </button>
        </div>

        <label className="faint" style={{ fontSize: 12.5 }}>
          Managed (writable) folders — changes are only allowed inside these
        </label>
        <div className="roots-list">
          {config.managed_roots.length === 0 && (
            <div className="faint" style={{ fontSize: 12 }}>
              Your home folder is always managed. Add extra roots (e.g. D:\) below.
            </div>
          )}
          {config.managed_roots.map((r) => (
            <div className="root-pill" key={r}>
              {r}
              <button
                className="icon-btn"
                style={{ marginLeft: "auto", background: "none", border: "none", color: "inherit" }}
                onClick={() => saveSettings({ managed_roots: config.managed_roots.filter((x) => x !== r) })}
              >
                <X size={13} />
              </button>
            </div>
          ))}
          <div className="inline" style={{ marginTop: 4 }}>
            <input
              type="text"
              placeholder="D:\Data"
              value={newRoot}
              onChange={(e) => setNewRoot(e.target.value)}
              style={{ flex: 1 }}
            />
            <button
              className="btn sm"
              disabled={!newRoot.trim()}
              onClick={() => {
                saveSettings({ managed_roots: [...config.managed_roots, newRoot.trim()] });
                setNewRoot("");
              }}
            >
              <Plus size={14} /> Add
            </button>
          </div>
        </div>

        <label className="faint" style={{ fontSize: 12.5, marginTop: 18, display: "block" }}>
          Protected locations — never modifiable
        </label>
        <div className="roots-list">
          {config.protected_roots.map((r) => (
            <div className="root-pill protected" key={r}>
              <FolderLock size={13} />
              {r}
            </div>
          ))}
        </div>
      </Accordion>

      {/* Data & Community ------------------------------------------------- */}
      <Accordion icon={<Database size={17} />} title="Data & Community" sub="all local, no cloud">
        <div className="hint">Everything is stored locally on this machine.</div>
        <div className="roots-list">
          <div className="root-pill">{config.app_root}</div>
          <div className="root-pill">{config.db_path}</div>
        </div>
        <div className="inline" style={{ marginTop: 14, alignItems: "center", gap: 10 }}>
          <button className="btn sm primary" onClick={() => api.openUrl(DISCORD_URL)}>
            <MessageCircle size={14} /> Join our Discord
          </button>
          <span className="faint" style={{ fontSize: 12 }}>
            come say hi 🤍
          </span>
        </div>
        <div className="faint" style={{ marginTop: 12, fontSize: 12 }}>
          Pebble v{config.app_version} · a tiny local AI, made with 🤍
        </div>
      </Accordion>

      {/* Extensions ------------------------------------------------------- */}
      <Accordion icon={<Puzzle size={17} />} title="Extensions" sub="opt-in abilities">
        <div className="hint">Extra abilities, off by default. Turn on what you'd like Pebble to do.</div>

        <ExtToggle
          label="Content Search"
          desc="Search inside your files (text, code, notes) by what they contain, not just the filename."
          on={config.ext_content_search}
          onToggle={() => saveSettings({ ext_content_search: !config.ext_content_search })}
        />
        <ExtToggle
          label="OCR — read images"
          desc="Pull text out of screenshots and photos using Windows' built-in OCR engine."
          on={config.ext_ocr}
          onToggle={() => saveSettings({ ext_ocr: !config.ext_ocr })}
        />
        <ExtToggle
          label="Duplicate Cleaner"
          desc="Find duplicate files and send the extra copies to the recoverable Trash (keeps one)."
          on={config.ext_dedupe}
          onToggle={() => saveSettings({ ext_dedupe: !config.ext_dedupe })}
        />

        <div className="section-label" style={{ marginTop: 16 }}>
          On the roadmap
        </div>
        <div className="plugin-grid">
          {plugins.map((p) => (
            <div className="plugin-card" key={p.id}>
              <div className="pn">
                {p.name}
                <span className="soon">soon</span>
              </div>
              <div className="pd">{p.description}</div>
            </div>
          ))}
        </div>
      </Accordion>
    </div>
  );
}

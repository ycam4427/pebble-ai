import { useEffect, useRef } from "react";
import { Check, MessageCircle, Pencil, Trash2 } from "lucide-react";
import { useStore } from "../../store/appStore";
import MessageBubble from "./MessageBubble";
import Composer from "./Composer";
import Pebble from "../Pebble";
import { Dropdown } from "../Dropdown";
import { chatReadiness } from "../../lib/readiness";

function greeting(name?: string) {
  const n = name?.trim();
  const who = n ? ` ${n}` : "";
  const h = new Date().getHours();
  if (h < 5) return `Still up${who}? 🤍 I'm right here.`;
  if (h < 12) return `Morning${who} 🤍 what's on your mind?`;
  if (h < 17) return `Afternoon${who} 🤍 how's it going?`;
  if (h < 22) return `Evening${who} 🤍 how was your day?`;
  return `Late night${who}? 🤍 I'm here.`;
}

function TypingTurn() {
  return (
    <div className="turn assistant">
      <div className="turn-body">
        <div className="who">Pebble</div>
        <div className="bubble">
          <div className="typing">
            <span />
            <span />
            <span />
          </div>
        </div>
      </div>
    </div>
  );
}

/** A gentle banner when Pebble can't think yet (Ollama down / no model). */
function ReadinessBanner() {
  const ollama = useStore((s) => s.ollama);
  const models = useStore((s) => s.models);
  const modelsLoaded = useStore((s) => s.modelsLoaded);
  const config = useStore((s) => s.config);
  const setTab = useStore((s) => s.setTab);
  const refreshOllama = useStore((s) => s.refreshOllama);
  const refreshModels = useStore((s) => s.refreshModels);

  const { banner } = chatReadiness(ollama, models, modelsLoaded, config);
  if (!banner) return null;

  const tone =
    banner.tone === "error"
      ? { background: "var(--accent-soft)", borderColor: "var(--bad)", color: "var(--text)" }
      : { background: "var(--accent-soft)", borderColor: "var(--accent)", color: "var(--text)" };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        margin: "10px 14px 0",
        padding: "10px 14px",
        border: "1px solid",
        borderRadius: 12,
        fontSize: 13,
        ...tone,
      }}
    >
      <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 2 }}>
        <b>{banner.title}</b>
        <span style={{ opacity: 0.85 }}>{banner.body}</span>
      </div>
      {banner.kind === "ollama" ? (
        <button
          className="btn sm"
          onClick={() => {
            refreshOllama();
            refreshModels();
          }}
        >
          Retry
        </button>
      ) : (
        <button className="btn sm" onClick={() => setTab("settings")}>
          Open Settings
        </button>
      )}
    </div>
  );
}

export default function ChatView() {
  const {
    turns,
    sending,
    streamingId,
    config,
    conversations,
    conversationId,
    selectConversation,
    removeConversation,
    openRename,
    askPebble,
  } = useStore();
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [turns, sending]);

  const currentTitle = conversations.find((c) => c.id === conversationId)?.title ?? "New chat";

  return (
    <div className="room">
      <div className="room-bar">
        <div className="room-title">🪨 Pebble's room</div>
        <div className="room-tools">
          {conversationId && (
            <button
              className="icon-btn"
              title="Rename this chat"
              onClick={() => openRename(conversationId, currentTitle)}
            >
              <Pencil size={14} />
            </button>
          )}
          {conversations.length > 1 && (
            <Dropdown
              trigger={
                <>
                  <MessageCircle size={14} /> chats
                </>
              }
            >
              {(close) =>
                conversations.map((c) => (
                  <div
                    key={c.id}
                    className={`menu-item ${c.id === conversationId ? "active" : ""}`}
                    style={{ display: "flex" }}
                  >
                    <button
                      onClick={() => {
                        selectConversation(c.id);
                        close();
                      }}
                      style={{
                        flex: 1,
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        background: "none",
                        border: "none",
                        color: "inherit",
                        overflow: "hidden",
                        textAlign: "left",
                      }}
                    >
                      <MessageCircle size={13} />
                      <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {c.title}
                      </span>
                      {c.id === conversationId && <Check size={12} />}
                    </button>
                    <span
                      className="ci"
                      title="Rename"
                      onClick={(e) => {
                        e.stopPropagation();
                        close();
                        openRename(c.id, c.title);
                      }}
                    >
                      <Pencil size={12} />
                    </span>
                    <span
                      className="ci danger"
                      title="Delete"
                      onClick={(e) => {
                        e.stopPropagation();
                        removeConversation(c.id);
                      }}
                    >
                      <Trash2 size={12} />
                    </span>
                  </div>
                ))
              }
            </Dropdown>
          )}
        </div>
      </div>

      <ReadinessBanner />

      <div className="transcript">
        {turns.length === 0 && (
          <div className="empty" style={{ margin: "auto" }}>
            <Pebble size={72} float />
            <div style={{ marginTop: 12, color: "var(--text-dim)", fontSize: 15 }}>
              {greeting(config?.user_name)}
            </div>
            <div className="notice" style={{ marginTop: 8 }}>
              Pebble's a small local AI — he means well but can be wrong. Be gentle 🤍
            </div>
            <button
              onClick={() => askPebble()}
              disabled={sending}
              title="Pebble will ask you something"
              style={{
                marginTop: 16,
                padding: "9px 18px",
                borderRadius: 999,
                border: "1px solid var(--accent)",
                background: "var(--accent-soft)",
                color: "var(--text)",
                cursor: sending ? "default" : "pointer",
                fontWeight: 600,
                fontSize: 13,
                opacity: sending ? 0.6 : 1,
              }}
            >
              🪨 Let Pebble ask you something
            </button>
          </div>
        )}
        {turns.map((t) => (
          <MessageBubble key={t.id} turn={t} />
        ))}
        {sending && !turns.some((t) => t.id === streamingId && t.content.length > 0) && (
          <TypingTurn />
        )}
        <div ref={endRef} />
      </div>

      <Composer />
    </div>
  );
}

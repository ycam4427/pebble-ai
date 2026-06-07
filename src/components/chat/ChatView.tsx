import { useEffect, useRef } from "react";
import { Check, MessageCircle, Pencil, Trash2 } from "lucide-react";
import { useStore } from "../../store/appStore";
import MessageBubble from "./MessageBubble";
import Composer from "./Composer";
import Pebble from "../Pebble";
import { Dropdown } from "../Dropdown";

function greeting(name?: string) {
  const n = name?.trim();
  return n
    ? `Hi ${n} 🤍 what can I help you tidy today?`
    : "Hi there 🤍 what can I help you tidy today?";
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

export default function ChatView() {
  const {
    turns,
    sending,
    config,
    conversations,
    conversationId,
    selectConversation,
    removeConversation,
    openRename,
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
          </div>
        )}
        {turns.map((t) => (
          <MessageBubble key={t.id} turn={t} />
        ))}
        {sending && <TypingTurn />}
        <div ref={endRef} />
      </div>

      <Composer />
    </div>
  );
}

import { useState } from "react";
import { Send, Sparkles, Wrench } from "lucide-react";
import { useStore } from "../../store/appStore";
import Pebble from "../Pebble";

const SUGGESTIONS = [
  "Clean out my Downloads",
  "Find my biggest files",
  "Show my storage usage",
  "Find duplicate files",
  "Find old files I haven't opened",
];

function autoGrow(el: HTMLTextAreaElement) {
  el.style.height = "auto";
  el.style.height = `${Math.min(el.scrollHeight, 150)}px`;
}

export default function Composer() {
  const send = useStore((s) => s.send);
  const sending = useStore((s) => s.sending);
  const turns = useStore((s) => s.turns);
  const chatMode = useStore((s) => s.chatMode);
  const setChatMode = useStore((s) => s.setChatMode);
  const [text, setText] = useState("");

  const submit = () => {
    const v = text.trim();
    if (!v || sending) return;
    setText("");
    send(v);
  };

  return (
    <div className="composer">
      <div className="mode-row">
        <div className="mode-switch">
          <button
            className={`mode-opt ${chatMode === "do" ? "active" : ""}`}
            onClick={() => setChatMode("do")}
          >
            <Wrench size={13} /> Chat &amp; Do
          </button>
          <button
            className={`mode-opt ${chatMode === "plan" ? "active" : ""}`}
            onClick={() => setChatMode("plan")}
          >
            <Sparkles size={13} /> Planning
          </button>
        </div>
      </div>

      {chatMode === "plan" && (
        <div className="plan-disclaimer">
          🧭 <b>Planning mode</b> — Pebble only talks it through and won't change any files. Because he's
          a small, local AI he usually needs <b>more detail than big assistants</b>, so give him specifics.
          Switch to <b>Chat &amp; Do</b> when you're ready to carry the plan out.
        </div>
      )}

      <div className="composer-row">
        <div className={`composer-peb ${sending ? "thinking" : ""}`} title="Pebble's right here 🪨">
          <Pebble size={42} float />
        </div>
        <div className="composer-inner">
          <textarea
            rows={1}
            value={text}
            placeholder={
              chatMode === "plan"
                ? "Let's plan something together…"
                : "Tell Pebble what to find, organize, or tidy…"
            }
            onChange={(e) => {
              setText(e.target.value);
              autoGrow(e.target);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                submit();
              }
            }}
          />
          <button className="send-btn" onClick={submit} disabled={!text.trim() || sending}>
            <Send size={17} />
          </button>
        </div>
      </div>

      {turns.length === 0 && chatMode === "do" && (
        <div className="suggestions">
          {SUGGESTIONS.map((s) => (
            <button key={s} className="chip" onClick={() => setText(s)}>
              {s}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

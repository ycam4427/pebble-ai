import { useState } from "react";
import { Image as ImageIcon, MessageCircle, Send, Sparkles, Square, Wrench } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../../store/appStore";
import Pebble from "../Pebble";
import { chatReadiness } from "../../lib/readiness";

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
  const stop = useStore((s) => s.stop);
  const readImage = useStore((s) => s.readImage);
  const sending = useStore((s) => s.sending);
  const turns = useStore((s) => s.turns);
  const chatMode = useStore((s) => s.chatMode);
  const setChatMode = useStore((s) => s.setChatMode);
  const ollama = useStore((s) => s.ollama);
  const models = useStore((s) => s.models);
  const modelsLoaded = useStore((s) => s.modelsLoaded);
  const config = useStore((s) => s.config);
  const { canSend } = chatReadiness(ollama, models, modelsLoaded, config);
  const [text, setText] = useState("");

  const submit = () => {
    const v = text.trim();
    if (!v || sending || !canSend) return;
    setText("");
    send(v);
  };

  const attach = async () => {
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"] },
        ],
      });
      if (typeof path === "string") readImage(path);
    } catch {
      /* cancelled or unavailable */
    }
  };

  return (
    <div className="composer">
      <div className="mode-row">
        <div className="mode-switch">
          <button
            className={`mode-opt ${chatMode === "chat" ? "active" : ""}`}
            onClick={() => setChatMode("chat")}
          >
            <MessageCircle size={13} /> Chat
          </button>
          <button
            className={`mode-opt ${chatMode === "do" ? "active" : ""}`}
            onClick={() => setChatMode("do")}
          >
            <Wrench size={13} /> Do
          </button>
          <button
            className={`mode-opt ${chatMode === "plan" ? "active" : ""}`}
            onClick={() => setChatMode("plan")}
          >
            <Sparkles size={13} /> Plan
          </button>
        </div>
      </div>

      <div className="composer-row">
        <div className={`composer-peb ${sending ? "thinking" : ""}`} title="Pebble's right here 🪨">
          <Pebble size={42} float />
        </div>
        <div className="composer-inner">
          <textarea
            rows={1}
            value={text}
            placeholder={
              !canSend
                ? "Pebble's not ready yet — check the note above ☝️"
                : chatMode === "plan"
                  ? "Let's plan something together…"
                  : chatMode === "chat"
                    ? "Talk to Pebble about anything…"
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
          <button
            className="attach-btn"
            onClick={attach}
            disabled={sending}
            title="Add an image for Pebble to read (OCR)"
          >
            <ImageIcon size={18} />
          </button>
          {sending ? (
            <button className="send-btn" onClick={() => stop()} title="Stop generating">
              <Square size={15} />
            </button>
          ) : (
            <button
              className="send-btn"
              onClick={submit}
              disabled={!text.trim() || !canSend}
              title={
                !canSend
                  ? "Pebble isn't ready yet — see the note above"
                  : !text.trim()
                    ? "Type a message first"
                    : "Send"
              }
            >
              <Send size={17} />
            </button>
          )}
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

import { useEffect, useState } from "react";
import { useStore } from "../store/appStore";

export default function RenameModal() {
  const target = useStore((s) => s.renameTarget);
  const close = useStore((s) => s.closeRename);
  const rename = useStore((s) => s.renameConversation);
  const [text, setText] = useState("");

  useEffect(() => {
    setText(target?.title ?? "");
  }, [target]);

  if (!target) return null;
  const save = () => {
    rename(target.id, text);
    close();
  };

  return (
    <div className="overlay" onClick={close}>
      <div className="modal" style={{ width: 380 }} onClick={(e) => e.stopPropagation()}>
        <h3>Rename chat</h3>
        <input
          className="confirm-input"
          autoFocus
          value={text}
          maxLength={60}
          style={{ fontFamily: "var(--font)", letterSpacing: "normal" }}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") save();
            if (e.key === "Escape") close();
          }}
        />
        <div className="actions-row" style={{ marginTop: 14 }}>
          <button className="btn ghost" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" onClick={save}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

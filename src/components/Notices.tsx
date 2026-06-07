import { MessageCircle, X } from "lucide-react";
import { useStore } from "../store/appStore";
import * as api from "../lib/ipc";
import { DISCORD_URL } from "../lib/constants";
import Pebble from "./Pebble";

// Small dismissible cards that appear on launch (Discord invite + a model nudge
// when your PC can run something smarter).
export default function Notices() {
  const notices = useStore((s) => s.notices);
  const dismiss = useStore((s) => s.dismissNotice);
  const setTab = useStore((s) => s.setTab);
  if (!notices.length) return null;

  return (
    <div className="notices">
      {notices.map((n) => (
        <div className="notice-card" key={n.id}>
          <button className="nc-x" onClick={() => dismiss(n.id)} aria-label="Dismiss">
            <X size={13} />
          </button>
          <div className="nc-head">
            <span className="nc-ico">
              {n.kind === "discord" ? <MessageCircle size={17} /> : <Pebble size={26} />}
            </span>
            <span className="nc-title">{n.title}</span>
          </div>
          <div className="nc-text">{n.body}</div>
          <div className="nc-actions">
            {n.kind === "discord" ? (
              <button
                className="btn sm primary"
                onClick={() => {
                  api.openUrl(DISCORD_URL);
                  dismiss(n.id);
                }}
              >
                Join
              </button>
            ) : (
              <button
                className="btn sm primary"
                onClick={() => {
                  setTab("settings");
                  dismiss(n.id);
                }}
              >
                See models
              </button>
            )}
            <button className="btn sm ghost" onClick={() => dismiss(n.id)}>
              Maybe later
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

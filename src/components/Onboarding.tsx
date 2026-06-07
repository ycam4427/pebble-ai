import { useEffect, useState } from "react";
import { useStore } from "../store/appStore";
import pebble from "../assets/pebble.png";

// Letter-by-letter fade-in. Spaces are preserved (white-space: pre on .char).
function FadeText({ text, start = 0, className = "" }: { text: string; start?: number; className?: string }) {
  return (
    <div className={className}>
      {Array.from(text).map((ch, i) => (
        <span key={i} className="char" style={{ animationDelay: `${start + i * 0.035}s` }}>
          {ch}
        </span>
      ))}
    </div>
  );
}

export default function Onboarding() {
  const completeOnboarding = useStore((s) => s.completeOnboarding);
  const [phase, setPhase] = useState<"greet" | "name">("greet");
  const [out, setOut] = useState(false);
  const [name, setName] = useState("");

  // Auto-advance from the greeting to the name prompt.
  useEffect(() => {
    if (phase !== "greet") return;
    const t = setTimeout(() => {
      setOut(true);
      setTimeout(() => {
        setPhase("name");
        setOut(false);
      }, 500);
    }, 5400);
    return () => clearTimeout(t);
  }, [phase]);

  const toName = () => {
    setOut(true);
    setTimeout(() => {
      setPhase("name");
      setOut(false);
    }, 420);
  };
  const finish = () => completeOnboarding(name.trim() || "friend");

  return (
    <div className="onboard">
      <div className="ob-inner">
        <img src={pebble} className="ob-peb" alt="Pebble" />
        {phase === "greet" ? (
          <div className={`ob-phase ${out ? "out" : ""}`}>
            <FadeText text="Meet Pebble," className="ob-line" start={0.2} />
            <FadeText text="your AI assistant." className="ob-line" start={1.0} />
            <FadeText
              text="He's going to be your friend for a while 🤍"
              className="ob-line small"
              start={2.4}
            />
            <button className="ob-skip" onClick={toName}>
              skip ›
            </button>
          </div>
        ) : (
          <div className="ob-phase">
            <FadeText text="Pebble wants to meet you too! ✨" className="ob-line" start={0.1} />
            <FadeText text="What should he call you?" className="ob-line small" start={1.0} />
            <form
              className="ob-name"
              onSubmit={(e) => {
                e.preventDefault();
                finish();
              }}
            >
              <input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="your name"
                maxLength={40}
              />
            </form>
            <button className="ob-cta" onClick={finish}>
              Nice to meet you{name.trim() ? `, ${name.trim()}` : ""}!
            </button>
            <div className="ob-note">
              <b>!</b> Pebble is a tiny AI running 100% on your computer — he tries his best, but he's
              little, so he can make mistakes and won't always be right. Please double-check anything
              important… and be gentle with him, okay? 🤍
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

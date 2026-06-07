import { useMemo } from "react";
import { useStore } from "../store/appStore";

export default function ThemeBackground() {
  const theme = useStore((s) => s.config?.theme ?? "pebble");

  const stars = useMemo(
    () =>
      Array.from({ length: 46 }, () => ({
        top: Math.random() * 100,
        left: Math.random() * 100,
        size: 1 + Math.random() * 2.4,
        dur: 2.6 + Math.random() * 4,
        delay: Math.random() * 5,
      })),
    [],
  );

  return (
    <div className="bg" aria-hidden>
      {theme === "stars" &&
        stars.map((s, i) => (
          <span
            key={i}
            className="star"
            style={{
              top: `${s.top}%`,
              left: `${s.left}%`,
              width: s.size,
              height: s.size,
              animationDuration: `${s.dur}s`,
              animationDelay: `${s.delay}s`,
            }}
          />
        ))}

      {theme === "liquidglass" && (
        <>
          <span className="orb" style={{ top: "-6%", left: "6%", width: 360, height: 360, background: "radial-gradient(circle, #3aa0ff, transparent 70%)", animationDuration: "24s" }} />
          <span className="orb" style={{ top: "38%", left: "58%", width: 440, height: 440, background: "radial-gradient(circle, #1f6dd6, transparent 70%)", animationDuration: "31s", animationDelay: "-6s" }} />
          <span className="orb" style={{ top: "62%", left: "-6%", width: 320, height: 320, background: "radial-gradient(circle, #6fd0ff, transparent 70%)", animationDuration: "27s", animationDelay: "-12s" }} />
          <span className="orb" style={{ top: "8%", left: "74%", width: 280, height: 280, background: "radial-gradient(circle, #8a7bff, transparent 70%)", animationDuration: "21s", animationDelay: "-3s" }} />
        </>
      )}

      {(theme === "pebble" || theme === "matcha" || theme === "cloud") && (
        <>
          <span className="blob" style={{ top: "-8%", left: "-4%", width: 380, height: 380, background: "radial-gradient(circle, var(--accent), transparent 70%)", animationDuration: "28s" }} />
          <span className="blob" style={{ top: "55%", left: "70%", width: 330, height: 330, background: "radial-gradient(circle, var(--accent-soft), transparent 70%)", animationDuration: "33s", animationDelay: "-8s", opacity: 0.7 }} />
        </>
      )}
    </div>
  );
}

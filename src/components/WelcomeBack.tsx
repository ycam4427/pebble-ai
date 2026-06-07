import { useEffect } from "react";
import { useStore } from "../store/appStore";
import pebble from "../assets/pebble.png";

// Shown briefly on every launch *after* the user has met Pebble. Auto-dismisses.
export default function WelcomeBack() {
  const name = useStore((s) => s.config?.user_name?.trim());
  const dismiss = useStore((s) => s.dismissWelcome);

  useEffect(() => {
    const t = setTimeout(dismiss, 2800);
    return () => clearTimeout(t);
  }, [dismiss]);

  return (
    <div className="onboard welcome" onClick={dismiss}>
      <div className="ob-inner">
        <img src={pebble} className="ob-peb" alt="Pebble" />
        <div className="ob-phase">
          <div className="ob-line">Welcome back{name ? `, ${name}` : ""}! 🤍</div>
          <div className="ob-line small">Pebble kept your spot warm and missed you a little ✨</div>
          <div className="faint" style={{ fontSize: 11, marginTop: 4 }}>(tap anywhere to continue)</div>
        </div>
      </div>
    </div>
  );
}

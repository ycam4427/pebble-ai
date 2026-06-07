import { X } from "lucide-react";
import { useStore } from "../store/appStore";

export default function Toast() {
  const error = useStore((s) => s.error);
  const setError = useStore((s) => s.setError);
  if (!error) return null;
  return (
    <div className="toast">
      <span>{error}</span>
      <button onClick={() => setError(null)} aria-label="Dismiss">
        <X size={16} />
      </button>
    </div>
  );
}

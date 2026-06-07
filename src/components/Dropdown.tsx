import { useState, type ReactNode } from "react";

// Minimal dropdown ("dropbar"). The trigger content renders inside a pill button;
// children receive a `close` callback.
export function Dropdown({
  trigger,
  children,
  align = "right",
}: {
  trigger: ReactNode;
  children: (close: () => void) => ReactNode;
  align?: "left" | "right";
}) {
  const [open, setOpen] = useState(false);
  const close = () => setOpen(false);
  return (
    <div className="menu">
      <button className="menu-btn" onClick={() => setOpen((o) => !o)}>
        {trigger}
      </button>
      {open && (
        <>
          <div className="backdrop" onClick={close} />
          <div className={`menu-pop ${align === "left" ? "left" : ""}`}>{children(close)}</div>
        </>
      )}
    </div>
  );
}

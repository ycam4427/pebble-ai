import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

// Collapsible "dropbar" section used to keep Settings minimal and clean.
export function Accordion({
  icon,
  title,
  sub,
  defaultOpen = false,
  children,
}: {
  icon: ReactNode;
  title: string;
  sub?: string;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className={`accordion ${open ? "open" : ""}`}>
      <button className="accordion-head" onClick={() => setOpen((o) => !o)}>
        <span className="ico">{icon}</span>
        <span>
          {title}
          {sub && <span className="sub"> · {sub}</span>}
        </span>
        <ChevronDown className="chev" size={18} />
      </button>
      {open && <div className="accordion-body">{children}</div>}
    </div>
  );
}

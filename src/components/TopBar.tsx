import { FolderOpen, History, Menu as MenuIcon, MessageCircle, Plus, Settings, Trash2 } from "lucide-react";
import { useStore, type Tab } from "../store/appStore";
import { Dropdown } from "./Dropdown";
import Pebble from "./Pebble";

const NAV: { id: Tab; label: string; icon: typeof MenuIcon }[] = [
  { id: "chat", label: "Pebble's room", icon: MessageCircle },
  { id: "files", label: "Files", icon: FolderOpen },
  { id: "trash", label: "Trash", icon: Trash2 },
  { id: "history", label: "Action history", icon: History },
  { id: "settings", label: "Settings", icon: Settings },
];

export default function TopBar() {
  const { tab, setTab, config, startNewConversation } = useStore();
  const current = NAV.find((n) => n.id === tab);
  const name = config?.user_name?.trim();

  return (
    <div className="topbar">
      <div className="brand">
        <Pebble size={30} className="peb" /> Pebble
      </div>
      <div className="spacer" />
      {name && <span className="hello">hi, {name} 🤍</span>}
      {tab === "chat" && (
        <button className="menu-btn" onClick={() => startNewConversation()} title="Start a new chat">
          <Plus size={15} /> New chat
        </button>
      )}
      <Dropdown
        trigger={
          <>
            <MenuIcon size={16} /> {current?.label ?? "Menu"}
          </>
        }
      >
        {(close) =>
          NAV.map((n) => (
            <button
              key={n.id}
              className={`menu-item ${tab === n.id ? "active" : ""}`}
              onClick={() => {
                setTab(n.id);
                close();
              }}
            >
              <n.icon size={16} /> {n.label}
            </button>
          ))
        }
      </Dropdown>
    </div>
  );
}

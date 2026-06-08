import { useEffect } from "react";
import { useStore } from "./store/appStore";
import ThemeBackground from "./components/ThemeBackground";
import TopBar from "./components/TopBar";
import Onboarding from "./components/Onboarding";
import WelcomeBack from "./components/WelcomeBack";
import Notices from "./components/Notices";
import Toast from "./components/Toast";
import ChatView from "./components/chat/ChatView";
import FilesView from "./components/files/FilesView";
import TrashView from "./components/trash/TrashView";
import HistoryView from "./components/history/HistoryView";
import SettingsView from "./components/settings/SettingsView";
import PlanModal from "./components/actions/PlanModal";
import RenameModal from "./components/RenameModal";

export default function App() {
  const init = useStore((s) => s.init);
  const tab = useStore((s) => s.tab);
  const config = useStore((s) => s.config);
  const activePlan = useStore((s) => s.activePlan);
  const greeted = useStore((s) => s.greeted);
  const dragging = useStore((s) => s.dragging);

  useEffect(() => {
    init();
  }, [init]);

  const needsOnboarding = config != null && !config.onboarded;
  const showWelcome = config != null && config.onboarded && !greeted;

  return (
    <>
      <ThemeBackground />
      <div className="stage">
        <TopBar />
        <div className="content">
          <div className="page">
            {tab === "chat" && <ChatView />}
            {tab === "files" && <FilesView />}
            {tab === "trash" && <TrashView />}
            {tab === "history" && <HistoryView />}
            {tab === "settings" && <SettingsView />}
          </div>
        </div>
      </div>
      {activePlan && tab !== "chat" && <PlanModal />}
      <RenameModal />
      {needsOnboarding && <Onboarding />}
      {showWelcome && <WelcomeBack />}
      {!needsOnboarding && <Notices />}
      <Toast />
      {dragging && (
        <div className="drop-overlay">
          <div className="drop-card">
            <div className="drop-plus">+</div>
            <div className="drop-title">Drop an image here</div>
            <div className="drop-sub">Pebble will read the text inside it 🪨</div>
          </div>
        </div>
      )}
    </>
  );
}

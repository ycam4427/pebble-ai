import { create } from "zustand";
import * as api from "../lib/ipc";
import { isSmallModel } from "../lib/constants";
import type {
  ActionLogEntry,
  Config,
  Conversation,
  ExecutionReport,
  ModelStatus,
  OllamaStatus,
  PluginInfo,
  QueryResult,
  GenStats,
  SystemInfo,
  TrashItem,
  ValidatedPlan,
  SettingsUpdate,
} from "../lib/types";

export interface Notice {
  id: string;
  kind: "discord" | "model";
  title: string;
  body: string;
  model?: string;
}

export type Tab = "chat" | "files" | "trash" | "history" | "settings";

export interface ChatTurn {
  id: string;
  role: "user" | "assistant";
  content: string;
  queryResults?: QueryResult[];
  plan?: ValidatedPlan | null;
  stats?: GenStats | null;
  handled?: boolean; // plan approved or rejected
  isError?: boolean;
}

interface AppStore {
  tab: Tab;
  setTab: (t: Tab) => void;
  chatMode: "do" | "plan";
  setChatMode: (m: "do" | "plan") => void;

  conversations: Conversation[];
  conversationId: string | null;
  turns: ChatTurn[];
  sending: boolean;
  error: string | null;
  setError: (e: string | null) => void;

  activePlan: ValidatedPlan | null;
  lastReport: ExecutionReport | null;
  greeted: boolean;

  config: Config | null;
  ollama: OllamaStatus | null;
  models: ModelStatus[];
  pulling: string | null;
  system: SystemInfo | null;
  notices: Notice[];
  dismissNotice: (id: string) => void;
  trash: TrashItem[];
  history: ActionLogEntry[];
  plugins: PluginInfo[];

  init: () => Promise<void>;
  loadConversations: () => Promise<void>;
  selectConversation: (id: string) => Promise<void>;
  startNewConversation: () => Promise<void>;
  removeConversation: (id: string) => Promise<void>;
  renameConversation: (id: string, title: string) => Promise<void>;
  renameTarget: { id: string; title: string } | null;
  openRename: (id: string, title: string) => void;
  closeRename: () => void;

  send: (content: string) => Promise<void>;
  approve: (planId: string, typed?: string) => Promise<void>;
  reject: (planId: string) => Promise<void>;
  proposeTrash: (paths: string[]) => Promise<void>;

  refreshTrash: () => Promise<void>;
  restore: (id: string) => Promise<void>;
  removeTrashItem: (id: string) => Promise<void>;
  emptyTrash: () => Promise<void>;

  refreshHistory: () => Promise<void>;
  undoLast: () => Promise<void>;
  undoOne: (id: string) => Promise<void>;

  refreshOllama: () => Promise<void>;
  refreshModels: () => Promise<void>;
  pullModel: (name: string) => Promise<void>;
  saveSettings: (u: SettingsUpdate) => Promise<void>;
  setTheme: (key: string) => Promise<void>;
  completeOnboarding: (name: string) => Promise<void>;
  dismissWelcome: () => void;
}

const uid = () =>
  typeof crypto !== "undefined" && crypto.randomUUID ? crypto.randomUUID() : String(Math.random());

export const useStore = create<AppStore>()((set, get) => ({
  tab: "chat",
  setTab: (t) => set({ tab: t }),
  chatMode: "do",
  setChatMode: (m) => set({ chatMode: m }),

  conversations: [],
  conversationId: null,
  turns: [],
  renameTarget: null,
  openRename: (id, title) => set({ renameTarget: { id, title } }),
  closeRename: () => set({ renameTarget: null }),
  sending: false,
  error: null,
  setError: (e) => set({ error: e }),

  activePlan: null,
  lastReport: null,
  greeted: false,

  config: null,
  ollama: null,
  models: [],
  pulling: null,
  system: null,
  notices: [],
  dismissNotice: (id) => set({ notices: get().notices.filter((n) => n.id !== id) }),
  trash: [],
  history: [],
  plugins: [],

  async init() {
    try {
      const [config, plugins] = await Promise.all([api.getConfig(), api.pluginsRoadmap()]);
      if (config) document.documentElement.dataset.theme = config.theme || "pebble";
      set({ config, plugins });
    } catch (e) {
      set({ error: String(e) });
    }
    try {
      const conv = await api.ensureConversation();
      set({ conversationId: conv.id });
      await get().selectConversation(conv.id);
    } catch (e) {
      set({ error: String(e) });
    }
    await get().loadConversations();
    await get().refreshOllama();
    get().refreshModels();

    // Hardware-aware welcome notifications. Discord shows regardless; the model
    // nudge is added only if detection succeeds and a better model fits.
    const notices: Notice[] = [
      {
        id: "discord",
        kind: "discord",
        title: "Join the Pebble Discord 💬",
        body: "Come hang out, share ideas, and get help — there's a little community for Pebble.",
      },
    ];
    try {
      const system = await api.systemInfo();
      const cfg = get().config;
      if (
        system &&
        cfg &&
        system.tier !== "light" &&
        system.recommended_model &&
        system.recommended_model !== cfg.model &&
        isSmallModel(cfg.model)
      ) {
        notices.push({
          id: "model",
          kind: "model",
          model: system.recommended_model,
          title: "Your PC can run a smarter Pebble ✨",
          body: `${system.reason} You could switch to ${system.recommended_model} in Settings.`,
        });
      }
      set({ system });
    } catch {
      /* detection is best-effort */
    }
    set({ notices });
  },

  async loadConversations() {
    try {
      set({ conversations: await api.listConversations() });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async selectConversation(id) {
    try {
      const msgs = await api.conversationMessages(id);
      const turns: ChatTurn[] = msgs.map((m) => ({
        id: m.id,
        role: m.role === "user" ? "user" : "assistant",
        content: m.content,
      }));
      set({ conversationId: id, turns, activePlan: null, lastReport: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async startNewConversation() {
    try {
      const conv = await api.newConversation();
      set({ conversationId: conv.id, turns: [], activePlan: null, lastReport: null });
      await get().loadConversations();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async removeConversation(id) {
    try {
      await api.deleteConversation(id);
      await get().loadConversations();
      if (get().conversationId === id) {
        const next = get().conversations[0];
        if (next) await get().selectConversation(next.id);
        else await get().startNewConversation();
      }
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async renameConversation(id, title) {
    try {
      await api.renameConversation(id, title.trim() || "New chat");
      await get().loadConversations();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async send(content) {
    const cid = get().conversationId;
    if (!cid || !content.trim() || get().sending) return;
    const firstMessage = get().turns.length === 0;
    const userTurn: ChatTurn = { id: uid(), role: "user", content };
    set({ turns: [...get().turns, userTurn], sending: true, error: null });
    try {
      const res = await api.sendMessage(cid, content, get().chatMode);
      const assistant: ChatTurn = {
        id: uid(),
        role: "assistant",
        content: res.message,
        queryResults: res.query_results,
        plan: res.plan,
        stats: res.stats,
      };
      set({
        turns: [...get().turns, assistant],
        activePlan: res.plan ?? get().activePlan,
      });
      get().loadConversations();
      // Let Pebble name a brand-new chat himself (like ChatGPT).
      if (firstMessage) {
        api
          .autoTitle(cid)
          .then(() => get().loadConversations())
          .catch(() => {});
      }
    } catch (e) {
      const assistant: ChatTurn = {
        id: uid(),
        role: "assistant",
        content: `⚠ ${String(e)}`,
        isError: true,
      };
      set({ turns: [...get().turns, assistant], error: String(e) });
    } finally {
      set({ sending: false });
    }
  },

  async approve(planId, typed) {
    try {
      const report = await api.approvePlan(planId, typed);
      const summary = `Done ✨ ${report.executed} action${report.executed === 1 ? "" : "s"} completed${
        report.failed ? `, ${report.failed} couldn't finish` : ""
      }. You can undo anytime from Action history 🤍`;
      const turns = get().turns.map((t) => (t.plan?.id === planId ? { ...t, handled: true } : t));
      turns.push({ id: uid(), role: "assistant", content: summary });
      set({
        lastReport: report,
        activePlan: get().activePlan?.id === planId ? null : get().activePlan,
        error: null,
        turns,
      });
      await Promise.all([get().refreshHistory(), get().refreshTrash()]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async reject(planId) {
    try {
      await api.rejectPlan(planId);
    } catch {
      /* ignore */
    }
    set({
      activePlan: get().activePlan?.id === planId ? null : get().activePlan,
      turns: get().turns.map((t) => (t.plan?.id === planId ? { ...t, handled: true } : t)),
    });
  },

  async proposeTrash(paths) {
    if (!paths.length) return;
    try {
      const plan = await api.proposeDelete(paths);
      set({ activePlan: plan, lastReport: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async refreshTrash() {
    try {
      set({ trash: await api.listTrash() });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async restore(id) {
    try {
      await api.restoreTrash(id);
      await get().refreshTrash();
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async removeTrashItem(id) {
    try {
      await api.deleteTrashItem(id);
      await get().refreshTrash();
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async emptyTrash() {
    try {
      await api.emptyTrash();
      await get().refreshTrash();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async refreshHistory() {
    try {
      set({ history: await api.listActionLog(200) });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async undoLast() {
    try {
      await api.undoLast();
      await Promise.all([get().refreshHistory(), get().refreshTrash()]);
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async undoOne(id) {
    try {
      await api.undoAction(id);
      await Promise.all([get().refreshHistory(), get().refreshTrash()]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async refreshOllama() {
    try {
      set({ ollama: await api.ollamaStatus() });
    } catch (e) {
      set({ ollama: { running: false, version: "", url: "" }, error: String(e) });
    }
  },
  async refreshModels() {
    try {
      set({ models: await api.listModels() });
    } catch {
      set({ models: [] });
    }
  },
  async pullModel(name) {
    set({ pulling: name, error: null });
    try {
      await api.pullModel(name);
      await get().refreshModels();
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ pulling: null });
    }
  },
  async saveSettings(u) {
    try {
      const config = await api.updateSettings(u);
      if (config) document.documentElement.dataset.theme = config.theme || "pebble";
      set({ config });
      if (u.ollama_url !== undefined) {
        await get().refreshOllama();
        get().refreshModels();
      }
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async setTheme(key) {
    document.documentElement.dataset.theme = key; // instant feedback
    await get().saveSettings({ theme: key });
  },

  async completeOnboarding(name) {
    await get().saveSettings({ user_name: name.trim(), onboarded: true });
    set({ greeted: true }); // don't also show "welcome back" right after meeting
  },

  dismissWelcome: () => set({ greeted: true }),
}));

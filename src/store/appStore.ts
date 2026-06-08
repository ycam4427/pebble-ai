import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import * as api from "../lib/ipc";
import { isSmallModel } from "../lib/constants";
import type {
  ActionLogEntry,
  Config,
  Conversation,
  ExecutionReport,
  MemoryItem,
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
  streaming?: boolean;
  question?: boolean; // a question Pebble asked the user
  chunks?: string[]; // streamed deltas, for the gentle type-in animation
}

interface AppStore {
  tab: Tab;
  setTab: (t: Tab) => void;
  chatMode: "chat" | "do" | "plan";
  setChatMode: (m: "chat" | "do" | "plan") => void;

  conversations: Conversation[];
  conversationId: string | null;
  turns: ChatTurn[];
  streamingId: string | null;
  dragging: boolean;
  sending: boolean;
  error: string | null;
  setError: (e: string | null) => void;

  activePlan: ValidatedPlan | null;
  lastReport: ExecutionReport | null;
  greeted: boolean;

  config: Config | null;
  ollama: OllamaStatus | null;
  models: ModelStatus[];
  modelsLoaded: boolean;
  pulling: string | null;
  system: SystemInfo | null;
  notices: Notice[];
  dismissNotice: (id: string) => void;
  trash: TrashItem[];
  history: ActionLogEntry[];
  plugins: PluginInfo[];
  memories: MemoryItem[];

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
  stop: () => Promise<void>;
  askPebble: () => Promise<void>;
  setDragging: (v: boolean) => void;
  readImage: (path: string) => Promise<void>;
  refreshMemory: () => Promise<void>;
  deleteMemoryItem: (id: string) => Promise<void>;
  clearAllMemory: () => Promise<void>;
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

let streamListenersAttached = false;

export const useStore = create<AppStore>()((set, get) => ({
  tab: "chat",
  setTab: (t) => set({ tab: t }),
  chatMode: "do",
  setChatMode: (m) => set({ chatMode: m }),

  conversations: [],
  conversationId: null,
  turns: [],
  streamingId: null,
  dragging: false,
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
  modelsLoaded: false,
  pulling: null,
  system: null,
  notices: [],
  dismissNotice: (id) => set({ notices: get().notices.filter((n) => n.id !== id) }),
  trash: [],
  history: [],
  plugins: [],
  memories: [],

  async init() {
    // Live token stream from the backend → grows the active assistant turn.
    if (!streamListenersAttached) {
      streamListenersAttached = true;
      listen<{ id: string; delta: string }>("chat:token", (e) => {
        const sid = get().streamingId;
        if (!sid) return;
        const turns = get().turns;
        const idx = turns.findIndex((t) => t.id === sid);
        if (idx === -1) {
          set({
            turns: [
              ...turns,
              {
                id: sid,
                role: "assistant",
                content: e.payload.delta,
                chunks: [e.payload.delta],
                streaming: true,
              },
            ],
          });
        } else {
          const copy = turns.slice();
          const t = copy[idx];
          copy[idx] = {
            ...t,
            content: t.content + e.payload.delta,
            chunks: [...(t.chunks ?? []), e.payload.delta],
          };
          set({ turns: copy });
        }
      });
      listen<{ id: string }>("chat:reset", () => {
        const sid = get().streamingId;
        if (!sid) return;
        set({
          turns: get().turns.map((t) => (t.id === sid ? { ...t, content: "", chunks: [] } : t)),
        });
      });
      // Native file drag-and-drop (images → OCR).
      try {
        getCurrentWebview().onDragDropEvent((event) => {
          const pl = event.payload as { type: string; paths?: string[] };
          if (pl.type === "enter" || pl.type === "over") {
            if (!get().dragging) set({ dragging: true });
          } else if (pl.type === "leave") {
            set({ dragging: false });
          } else if (pl.type === "drop") {
            set({ dragging: false });
            const paths = pl.paths ?? [];
            const img = paths.find((p) => /\.(png|jpe?g|bmp|gif|webp|tiff?)$/i.test(p));
            if (img) get().readImage(img);
            else if (paths.length) set({ error: "I can only read images right now 🪨" });
          }
        });
      } catch {
        /* not running under Tauri */
      }
    }
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
    get().refreshMemory();

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
    // Reserve the assistant turn's id; it's created on the first streamed token.
    const assistantId = uid();
    set({
      turns: [...get().turns, userTurn],
      sending: true,
      error: null,
      streamingId: assistantId,
    });
    try {
      const res = await api.sendMessage(cid, content, get().chatMode);
      const turns = get().turns.slice();
      const idx = turns.findIndex((t) => t.id === assistantId);
      const authoritative = !!res.message && res.message.trim().length > 0;
      const streamed = idx !== -1 ? turns[idx].content : "";
      const finalTurn: ChatTurn = {
        id: assistantId,
        role: "assistant",
        content: authoritative ? res.message : streamed,
        queryResults: res.query_results,
        plan: res.plan,
        stats: res.stats,
        streaming: false,
        chunks: undefined,
      };
      if (idx === -1) turns.push(finalTurn);
      else turns[idx] = { ...turns[idx], ...finalTurn };
      set({ turns, activePlan: res.plan ?? get().activePlan, streamingId: null });
      get().loadConversations();
      // Let Pebble name a brand-new chat himself (like ChatGPT).
      if (firstMessage) {
        api
          .autoTitle(cid)
          .then(() => get().loadConversations())
          .catch(() => {});
      }
      // Let Pebble quietly remember anything worth keeping (opt-in).
      if (get().config?.allow_memory && get().chatMode !== "plan") {
        api
          .extractMemories(cid)
          .then(() => get().refreshMemory())
          .catch(() => {});
      }
    } catch (e) {
      const turns = get().turns.slice();
      const idx = turns.findIndex((t) => t.id === assistantId);
      const errTurn: ChatTurn = {
        id: assistantId,
        role: "assistant",
        content: `⚠ ${String(e)}`,
        isError: true,
        streaming: false,
        chunks: undefined,
      };
      if (idx === -1) turns.push(errTurn);
      else turns[idx] = { ...turns[idx], ...errTurn };
      set({ turns, error: String(e), streamingId: null });
    } finally {
      set({ sending: false, streamingId: null });
    }
  },

  async stop() {
    try {
      await api.cancelGeneration();
    } catch {
      /* ignore */
    }
  },

  async askPebble() {
    const cid = get().conversationId;
    if (!cid || get().sending) return;
    set({ sending: true, error: null });
    try {
      const q = await api.pebbleQuestion(cid);
      set({
        turns: [...get().turns, { id: uid(), role: "assistant", content: q, question: true }],
      });
      get().loadConversations();
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ sending: false });
    }
  },

  async refreshMemory() {
    try {
      set({ memories: await api.listMemory() });
    } catch {
      /* ignore */
    }
  },
  async deleteMemoryItem(id) {
    try {
      await api.deleteMemory(id);
      await get().refreshMemory();
    } catch (e) {
      set({ error: String(e) });
    }
  },
  async clearAllMemory() {
    try {
      await api.clearMemory();
      await get().refreshMemory();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setDragging: (v) => set({ dragging: v }),
  async readImage(path) {
    const cid = get().conversationId;
    if (!cid || get().sending) return;
    const fname = path.split(/[\\/]/).pop() || "image";
    set({
      tab: "chat",
      turns: [...get().turns, { id: uid(), role: "user", content: `📷 ${fname}` }],
      sending: true,
      error: null,
    });
    try {
      const text = await api.readImage(cid, path);
      set({ turns: [...get().turns, { id: uid(), role: "assistant", content: text }] });
      get().loadConversations();
    } catch (e) {
      set({
        turns: [
          ...get().turns,
          { id: uid(), role: "assistant", content: `⚠ ${String(e)}`, isError: true },
        ],
        error: String(e),
      });
    } finally {
      set({ sending: false });
    }
  },

  async approve(planId, typed) {
    try {
      const report = await api.approvePlan(planId, typed);
      let summary =
        report.executed > 0
          ? `Done ✨ ${report.executed} action${report.executed === 1 ? "" : "s"} completed`
          : "Hmm — nothing went through";
      if (report.failed) summary += `, ${report.failed} couldn't finish`;
      summary += report.executed > 0 ? ". You can undo anytime from Action history 🤍" : ".";
      if (report.failed && report.errors?.length) {
        const shown = report.errors
          .slice(0, 3)
          .map((e) => `• ${e}`)
          .join("\n");
        const more = report.errors.length > 3 ? `\n…and ${report.errors.length - 3} more` : "";
        summary += `\n\nWhat didn't work:\n${shown}${more}`;
      }
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
      set({ models: await api.listModels(), modelsLoaded: true });
    } catch {
      set({ models: [], modelsLoaded: true });
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

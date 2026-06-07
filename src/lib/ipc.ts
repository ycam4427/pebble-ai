// Typed wrappers over Tauri's `invoke`. Tauri maps camelCase JS keys to the
// snake_case Rust parameter names automatically.

import { invoke } from "@tauri-apps/api/core";
import type {
  ActionLogEntry,
  ChatResponse,
  Config,
  Conversation,
  ExecutionReport,
  FileEntry,
  FolderAnalysis,
  Location,
  Message,
  ModelStatus,
  OllamaStatus,
  PluginInfo,
  SettingsUpdate,
  StorageStats,
  SystemInfo,
  TrashItem,
  ValidatedPlan,
} from "./types";

// ---- chat ----
export const sendMessage = (conversationId: string, content: string, mode?: "do" | "plan") =>
  invoke<ChatResponse>("send_message", { conversationId, content, mode });

// ---- conversations ----
export const ensureConversation = () => invoke<Conversation>("ensure_conversation");
export const newConversation = (title?: string) =>
  invoke<Conversation>("new_conversation", { title });
export const listConversations = () => invoke<Conversation[]>("list_conversations");
export const conversationMessages = (conversationId: string) =>
  invoke<Message[]>("conversation_messages", { conversationId });
export const renameConversation = (conversationId: string, title: string) =>
  invoke<void>("rename_conversation", { conversationId, title });
export const autoTitle = (conversationId: string) =>
  invoke<string | null>("auto_title", { conversationId });
export const deleteConversation = (conversationId: string) =>
  invoke<void>("delete_conversation", { conversationId });

// ---- plans ----
export const approvePlan = (planId: string, typedConfirmation?: string) =>
  invoke<ExecutionReport>("approve_plan", { planId, typedConfirmation });
export const rejectPlan = (planId: string) => invoke<void>("reject_plan", { planId });
export const proposeDelete = (paths: string[]) =>
  invoke<ValidatedPlan>("propose_delete", { paths });
export const proposeMove = (paths: string[], destination: string) =>
  invoke<ValidatedPlan>("propose_move", { paths, destination });

// ---- trash ----
export const listTrash = () => invoke<TrashItem[]>("list_trash");
export const restoreTrash = (id: string) => invoke<void>("restore_trash", { id });
export const deleteTrashItem = (id: string) => invoke<void>("delete_trash_item", { id });
export const emptyTrash = () => invoke<number>("empty_trash");
export const cleanupTrash = () => invoke<number>("cleanup_trash");

// ---- action log / undo ----
export const listActionLog = (limit?: number) =>
  invoke<ActionLogEntry[]>("list_action_log", { limit });
export const undoLast = () => invoke<string | null>("undo_last");
export const undoAction = (id: string) => invoke<void>("undo_action", { id });
export const undoActions = (ids: string[]) => invoke<number>("undo_actions", { ids });

// ---- settings / ollama ----
export const getConfig = () => invoke<Config>("get_config");
export const updateSettings = (update: SettingsUpdate) =>
  invoke<Config>("update_settings", { update });
export const ollamaStatus = () => invoke<OllamaStatus>("ollama_status");
export const listModels = () => invoke<ModelStatus[]>("list_models");
export const pullModel = (name: string) => invoke<void>("pull_model", { name });
export const systemInfo = () => invoke<SystemInfo>("system_info");
export const openUrl = (url: string) => invoke<void>("open_url", { url });

// ---- files ----
export const fsListDir = (path?: string) => invoke<FileEntry[]>("fs_list_dir", { path });
export const fsAnalyze = (path: string) => invoke<FolderAnalysis>("fs_analyze", { path });
export const fsStorage = (path?: string) => invoke<StorageStats>("fs_storage", { path });

// ---- plugins / memory ----
export const pluginsRoadmap = () => invoke<PluginInfo[]>("plugins_roadmap");
export const frequentLocations = () => invoke<Location[]>("frequent_locations");

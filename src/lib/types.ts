// TypeScript mirrors of the Rust types serialized across the Tauri boundary.
// Keep field names in sync with the `#[derive(Serialize)]` structs in src-tauri.

export type Tier = "auto" | "confirm" | "high_risk";
export type OpKind = "move" | "rename" | "delete" | "execute" | "empty_recycle_bin";

export interface Operation {
  id: string;
  kind: OpKind;
  source: string;
  destination: string | null;
  size_bytes: number;
  is_dir: boolean;
  file_count: number;
  args: string[];
}

export type Verdict = { status: "approved" } | { status: "rejected"; reason: string };

export interface ValidatedOp {
  op: Operation;
  tier: Tier;
  verdict: Verdict;
  warnings: string[];
}

export interface ValidatedPlan {
  id: string;
  summary: string;
  ops: ValidatedOp[];
  max_tier: Tier;
  affected_locations: string[];
  requires_typed_confirmation: boolean;
  confirmation_phrase: string | null;
  move_count: number;
  rename_count: number;
  delete_count: number;
  execute_count: number;
  rejected_count: number;
  warnings: string[];
  rejected: string[];
}

export interface FileEntry {
  path: string;
  name: string;
  size: number;
  is_dir: boolean;
  modified: string | null;
  accessed: string | null;
  extension: string | null;
}

export interface CategoryStat {
  category: string;
  bytes: number;
  count: number;
}

export interface StorageStats {
  root: string;
  total_bytes: number;
  file_count: number;
  dir_count: number;
  by_category: CategoryStat[];
  largest: FileEntry[];
  truncated: boolean;
}

export interface DupGroup {
  hash: string;
  size: number;
  files: FileEntry[];
}

export interface WebResult {
  title: string;
  url: string;
  snippet: string;
}

export interface FolderAnalysis {
  root: string;
  total_bytes: number;
  file_count: number;
  dir_count: number;
  by_category: CategoryStat[];
  by_extension: CategoryStat[];
  recent: FileEntry[];
  truncated: boolean;
}

export type QueryResult =
  | { type: "large_files"; root: string; files: FileEntry[] }
  | { type: "duplicates"; root: string; groups: DupGroup[] }
  | { type: "stale_files"; root: string; days: number; files: FileEntry[] }
  | { type: "search_results"; root: string; query: string; files: FileEntry[] }
  | { type: "storage"; stats: StorageStats }
  | { type: "folder_analysis"; analysis: FolderAnalysis }
  | { type: "file_content"; path: string; preview: string; truncated: boolean }
  | { type: "summary"; path: string; summary: string }
  | { type: "web_results"; query: string; results: WebResult[] }
  | { type: "error"; message: string };

export interface GenStats {
  model: string;
  tokens: number;
  tokens_per_sec: number;
  total_ms: number;
}

export interface ChatResponse {
  message: string;
  query_results: QueryResult[];
  plan: ValidatedPlan | null;
  stats: GenStats | null;
}

export interface ExecutionReport {
  executed: number;
  failed: number;
  skipped: number;
  errors: string[];
  log_ids: string[];
}

export interface Conversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  created_at: string;
  actions_json: string | null;
}

export interface TrashItem {
  id: string;
  original_path: string;
  trash_path: string;
  name: string;
  size: number;
  is_dir: boolean;
  deleted_at: string;
  expires_at: string;
  restored_at: string | null;
}

export interface ActionLogEntry {
  id: string;
  plan_id: string | null;
  op_index: number;
  kind: string;
  tier: number;
  source: string;
  destination: string | null;
  status: string;
  error: string | null;
  executed_at: string;
  undone_at: string | null;
}

export interface Location {
  path: string;
  label: string | null;
  kind: string | null;
  use_count: number;
  last_used: string | null;
}

export interface KnownFolder {
  label: string;
  path: string;
}

export interface Config {
  model: string;
  ollama_url: string;
  retention_days: number;
  allow_execute: boolean;
  allow_web: boolean;
  managed_roots: string[];
  user_name: string;
  persona: string;
  about_you: string;
  theme: string;
  onboarded: boolean;
  app_version: string;
  app_root: string;
  trash_root: string;
  db_path: string;
  protected_roots: string[];
  known_folders: KnownFolder[];
}

export interface ModelStatus {
  name: string;
  size: number;
  parameter_size: string;
  quantization: string;
  family: string;
  loaded: boolean;
  vram_bytes: number;
}

export interface OllamaStatus {
  running: boolean;
  version: string;
  url: string;
}

export interface SystemInfo {
  ram_gb: number;
  cpu_cores: number;
  gpu_name: string;
  vram_gb: number;
  tier: string;
  recommended_model: string;
  reason: string;
}

export interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  enabled: boolean;
}

export interface SettingsUpdate {
  model?: string;
  ollama_url?: string;
  retention_days?: number;
  allow_execute?: boolean;
  allow_web?: boolean;
  managed_roots?: string[];
  user_name?: string;
  persona?: string;
  about_you?: string;
  theme?: string;
  onboarded?: boolean;
}

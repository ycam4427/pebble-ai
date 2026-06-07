//! Tauri command handlers — the thin bridge between the React UI and the trusted
//! core. Commands never bypass the validator: mutations always go
//! intent -> planner -> `safety::validate` -> pending map -> `executor` (on approve).

use crate::ai::{ModelStatus, OllamaClient};
use crate::executor::{self, ExecutionReport};
use crate::models::{
    Action, ActionLogEntry, Conversation, FileEntry, FolderAnalysis, GenStats, Location, Message,
    Operation, QueryResult, StorageStats, TrashItem,
};
use crate::plugin::{PluginInfo, PluginRegistry};
use crate::safety::ValidatedPlan;
use crate::state::{self, AppState, Config};
use crate::{ai, db, fsops, intent, memory, planner, safety, trash, undo};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ----------------------------------------------------------------- error helpers

fn to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}
fn lock_err<E>(_: E) -> String {
    "internal lock error".to_string()
}

// ----------------------------------------------------------------------- chat

#[derive(Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub query_results: Vec<QueryResult>,
    pub plan: Option<ValidatedPlan>,
    pub stats: Option<GenStats>,
}

/// Heuristic: the small model sometimes *claims* it did a task while emitting no
/// actions (which means nothing happened). Detect that so we can nudge it.
fn looks_like_false_completion(msg: &str) -> bool {
    let m = msg.to_lowercase();
    const CLAIMS: &[&str] = &[
        "done!", "done.", "all done", "i've ", "i have ", "i moved", "i organized", "i organised",
        "i deleted", "i renamed", "i cleaned", "i sorted", "i emptied", "has been moved",
        "have been moved", "taken care of", "all set", "finished", "successfully",
    ];
    CLAIMS.iter().any(|c| m.contains(c))
}

#[tauri::command]
pub async fn send_message(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    content: String,
    mode: Option<String>,
) -> Result<ChatResponse, String> {
    let planning = mode.as_deref() == Some("plan");

    // 1) Gather context under locks, then release before any await.
    let (messages, model, ollama, safety_cfg, allow_web) = {
        let conn = state.db.lock().map_err(lock_err)?;
        let cfg = state.config.lock().map_err(lock_err)?;
        db::insert_message(&conn, &conversation_id, "user", &content, None).map_err(to_str)?;
        let system = if planning {
            ai::planning_prompt(&cfg.user_name, &cfg.persona, &cfg.about_you)
        } else {
            ai::system_prompt(
                &state.platform.known_folders,
                &state.platform.common_locations,
                &cfg.trash_root,
                &cfg.user_name,
                &cfg.persona,
                &cfg.about_you,
                cfg.allow_web,
            )
        };
        let messages =
            memory::conversation_messages(&conn, &conversation_id, system, 8).map_err(to_str)?;
        let model = cfg.model.clone();
        let ollama = state.ollama.lock().map_err(lock_err)?.clone();
        let safety_cfg = state.safety_config(&cfg);
        (messages, model, ollama, safety_cfg, cfg.allow_web)
    };

    // 2) Ask the model. Plan mode = plain prose; Do mode = structured JSON.
    let outcome = ollama
        .chat(&model, &messages, !planning)
        .await
        .map_err(|e| e.to_string())?;

    // ---- Planning mode: just talk it through, never execute. ----
    if planning {
        let msg = outcome.content.trim().to_string();
        let conn = state.db.lock().map_err(lock_err)?;
        db::insert_message(&conn, &conversation_id, "assistant", &msg, None).map_err(to_str)?;
        return Ok(ChatResponse {
            message: msg,
            query_results: Vec::new(),
            plan: None,
            stats: Some(GenStats {
                model,
                tokens: outcome.eval_count,
                tokens_per_sec: outcome.tokens_per_sec(),
                total_ms: outcome.total_ms,
            }),
        });
    }

    // ---- Do mode ----
    let mut ai_plan = intent::parse(&outcome.content);
    let mut stats_outcome = outcome;

    // Corrective retry: if Pebble claimed it's "done" but produced no actions,
    // nudge it once to actually act or ask — words alone change nothing.
    if ai_plan.actions.is_empty() && looks_like_false_completion(&ai_plan.message) {
        let mut retry = messages.clone();
        retry.push(ai::ChatMessage {
            role: "assistant".into(),
            content: stats_outcome.content.clone(),
        });
        retry.push(ai::ChatMessage {
            role: "user".into(),
            content: "Hang on — you haven't actually done anything yet. You can only act by \
                      returning an \"actions\" array that I then approve; talking changes no files. \
                      If you know what to do, return the actions now. If you're missing details \
                      (which files? which destination folder?), or you don't know where something \
                      is, search for it or ask me. Do NOT say it's done."
                .into(),
        });
        if let Ok(o2) = ollama.chat(&model, &retry, true).await {
            ai_plan = intent::parse(&o2.content);
            stats_outcome = o2;
        }
    }

    // 3) Route actions (heavy filesystem reads run off the async thread).
    let mut query_results: Vec<QueryResult> = Vec::new();
    let mut ops: Vec<Operation> = Vec::new();
    for action in &ai_plan.actions {
        match planner::classify(action) {
            planner::Class::Read => {
                let a = action.clone();
                let qr = tauri::async_runtime::spawn_blocking(move || planner::run_query(&a))
                    .await
                    .map_err(to_str)?;
                query_results.push(qr);
            }
            planner::Class::Mutate => match planner::build_ops(action) {
                Ok(mut v) => ops.append(&mut v),
                Err(e) => query_results.push(QueryResult::Error {
                    message: e.to_string(),
                }),
            },
            planner::Class::Web => {
                if let Action::WebSearch { query } = action {
                    if !allow_web {
                        query_results.push(QueryResult::Error {
                            message: "Web search is off. You can turn it on in Settings → Safety \
                                      (Pebble is local-first, so the web is opt-in)."
                                .into(),
                        });
                    } else {
                        match crate::ai::web::search(query, 6).await {
                            Ok(results) => query_results.push(QueryResult::WebResults {
                                query: query.clone(),
                                results,
                            }),
                            Err(e) => query_results.push(QueryResult::Error {
                                message: format!("Web search failed: {e}"),
                            }),
                        }
                    }
                }
            }
            planner::Class::Summarize => {
                if let Action::SummarizeDocument { path } = action {
                    let p = planner::resolve_path(path);
                    match fsops::extract_text(&p, 24 * 1024) {
                        Ok((text, _)) => match ollama.summarize(&model, &text).await {
                            Ok(summary) => query_results.push(QueryResult::Summary {
                                path: p.display().to_string(),
                                summary,
                            }),
                            Err(e) => query_results.push(QueryResult::Error {
                                message: format!("Couldn't summarize: {e}"),
                            }),
                        },
                        Err(e) => query_results.push(QueryResult::Error {
                            message: format!("Couldn't read document: {e}"),
                        }),
                    }
                }
            }
        }
    }

    // 4) Validate mutations into a sealed plan; keep it server-side only.
    let plan = if ops.is_empty() {
        None
    } else {
        let vp = safety::validate(ai_plan.message.clone(), ops, &safety_cfg);
        state
            .pending
            .lock()
            .map_err(lock_err)?
            .insert(vp.id().to_string(), vp.clone());
        Some(vp)
    };

    // 5) Persist assistant turn + remember touched locations.
    {
        let conn = state.db.lock().map_err(lock_err)?;
        let actions_json = serde_json::to_string(&ai_plan.actions).ok();
        db::insert_message(
            &conn,
            &conversation_id,
            "assistant",
            &ai_plan.message,
            actions_json.as_deref(),
        )
        .map_err(to_str)?;
        if let Some(vp) = &plan {
            for vo in vp.approved_ops() {
                if let Some(parent) = Path::new(&vo.op.source).parent() {
                    memory::remember_location(&conn, &parent.display().to_string(), None);
                }
            }
        }
    }

    Ok(ChatResponse {
        message: ai_plan.message,
        query_results,
        plan,
        stats: Some(GenStats {
            model,
            tokens: stats_outcome.eval_count,
            tokens_per_sec: stats_outcome.tokens_per_sec(),
            total_ms: stats_outcome.total_ms,
        }),
    })
}

// ---------------------------------------------------------------- conversations

#[tauri::command]
pub fn ensure_conversation(state: tauri::State<'_, AppState>) -> Result<Conversation, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    if let Some(c) = db::list_conversations(&conn).map_err(to_str)?.into_iter().next() {
        return Ok(c);
    }
    db::create_conversation(&conn, "New chat").map_err(to_str)
}

#[tauri::command]
pub fn new_conversation(
    state: tauri::State<'_, AppState>,
    title: Option<String>,
) -> Result<Conversation, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::create_conversation(&conn, &title.unwrap_or_else(|| "New chat".to_string())).map_err(to_str)
}

#[tauri::command]
pub fn list_conversations(state: tauri::State<'_, AppState>) -> Result<Vec<Conversation>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::list_conversations(&conn).map_err(to_str)
}

#[tauri::command]
pub fn conversation_messages(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::list_messages(&conn, &conversation_id).map_err(to_str)
}

#[tauri::command]
pub fn rename_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::rename_conversation(&conn, &conversation_id, &title).map_err(to_str)
}

fn clean_title(s: &str) -> String {
    let line = s.trim().lines().next().unwrap_or("").trim();
    let line = line.trim_matches(|c| c == '"' || c == '\'' || c == '.' || c == ':' || c == '#');
    let words: Vec<&str> = line.split_whitespace().take(6).collect();
    words.join(" ").chars().take(48).collect::<String>().trim().to_string()
}

fn fallback_title(user_msg: &str) -> String {
    let t: String = user_msg.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(48).collect();
    if t.trim().is_empty() {
        "New chat".to_string()
    } else {
        t.trim().to_string()
    }
}

/// Auto-name a brand-new conversation from its first message (like ChatGPT).
/// No-op if the chat already has a custom title.
#[tauri::command]
pub async fn auto_title(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<String>, String> {
    let (first_user, model, ollama) = {
        let conn = state.db.lock().map_err(lock_err)?;
        let conv = db::get_conversation(&conn, &conversation_id).map_err(to_str)?;
        let is_default = conv
            .map(|c| c.title == "New chat" || c.title.trim().is_empty())
            .unwrap_or(false);
        if !is_default {
            return Ok(None);
        }
        let first = db::list_messages(&conn, &conversation_id)
            .map_err(to_str)?
            .into_iter()
            .find(|m| m.role == "user")
            .map(|m| m.content);
        let first = match first {
            Some(f) => f,
            None => return Ok(None),
        };
        let cfg = state.config.lock().map_err(lock_err)?;
        let model = cfg.model.clone();
        let ollama = state.ollama.lock().map_err(lock_err)?.clone();
        (first, model, ollama)
    };

    let prompt = format!(
        "Give a very short title (2 to 4 words) for a chat that begins with this message. \
         Reply with ONLY the title — no quotes, no trailing punctuation.\n\nMessage: {first_user}"
    );
    let messages = vec![ai::ChatMessage {
        role: "user".into(),
        content: prompt,
    }];
    let titled = match ollama.chat(&model, &messages, false).await {
        Ok(o) => clean_title(&o.content),
        Err(_) => String::new(),
    };
    let title = if titled.is_empty() {
        fallback_title(&first_user)
    } else {
        titled
    };

    {
        let conn = state.db.lock().map_err(lock_err)?;
        db::rename_conversation(&conn, &conversation_id, &title).map_err(to_str)?;
    }
    Ok(Some(title))
}

#[tauri::command]
pub fn delete_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::delete_conversation(&conn, &conversation_id).map_err(to_str)
}

// ---------------------------------------------------------------- plan actions

#[tauri::command]
pub fn approve_plan(
    state: tauri::State<'_, AppState>,
    plan_id: String,
    typed_confirmation: Option<String>,
) -> Result<ExecutionReport, String> {
    let plan = state.pending.lock().map_err(lock_err)?.remove(&plan_id);
    let plan = plan.ok_or_else(|| "This plan is no longer available.".to_string())?;

    if !plan.confirmation_satisfied(typed_confirmation.as_deref()) {
        let phrase = plan.confirmation_phrase().unwrap_or("CONFIRM").to_string();
        // Put it back so the user can retry with the correct phrase.
        state
            .pending
            .lock()
            .map_err(lock_err)?
            .insert(plan_id, plan);
        return Err(format!("Please type \"{phrase}\" exactly to confirm."));
    }

    let cfg = state.config.lock().map_err(lock_err)?.clone();
    let trash_cfg = state.trash_config(&cfg);
    let conn = state.db.lock().map_err(lock_err)?;
    Ok(executor::execute(&plan, &conn, &trash_cfg))
}

#[tauri::command]
pub fn reject_plan(state: tauri::State<'_, AppState>, plan_id: String) -> Result<(), String> {
    state.pending.lock().map_err(lock_err)?.remove(&plan_id);
    Ok(())
}

/// Build + validate a plan from explicit UI actions (Files view), returning the
/// sealed plan for the confirmation panel.
fn propose(state: &AppState, summary: &str, actions: Vec<Action>) -> Result<ValidatedPlan, String> {
    let cfg = state.config.lock().map_err(lock_err)?.clone();
    let safety_cfg = state.safety_config(&cfg);
    let mut ops: Vec<Operation> = Vec::new();
    for a in &actions {
        ops.extend(planner::build_ops(a).map_err(to_str)?);
    }
    let vp = safety::validate(summary.to_string(), ops, &safety_cfg);
    state
        .pending
        .lock()
        .map_err(lock_err)?
        .insert(vp.id().to_string(), vp.clone());
    Ok(vp)
}

#[tauri::command]
pub fn propose_delete(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ValidatedPlan, String> {
    let actions = paths
        .into_iter()
        .map(|p| Action::DeleteFile { path: p })
        .collect();
    propose(&state, "Move selected items to Trash", actions)
}

#[tauri::command]
pub fn propose_move(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
    destination: String,
) -> Result<ValidatedPlan, String> {
    let actions = paths
        .into_iter()
        .map(|p| Action::MoveFile {
            source: p,
            destination: destination.clone(),
        })
        .collect();
    propose(&state, "Move selected items", actions)
}

// --------------------------------------------------------------------- trash

#[tauri::command]
pub fn list_trash(state: tauri::State<'_, AppState>) -> Result<Vec<TrashItem>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::list_trash(&conn).map_err(to_str)
}

#[tauri::command]
pub fn restore_trash(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    trash::restore(&conn, &id).map(|_| ()).map_err(to_str)
}

#[tauri::command]
pub fn delete_trash_item(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    trash::delete_one(&conn, &id).map_err(to_str)
}

#[tauri::command]
pub fn empty_trash(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    trash::empty(&conn).map_err(to_str)
}

#[tauri::command]
pub fn cleanup_trash(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    trash::cleanup(&conn).map_err(to_str)
}

// ----------------------------------------------------------------- action log

#[tauri::command]
pub fn list_action_log(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<ActionLogEntry>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::list_action_log(&conn, limit.unwrap_or(200)).map_err(to_str)
}

#[tauri::command]
pub fn undo_last(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    undo::undo_last(&conn).map_err(to_str)
}

#[tauri::command]
pub fn undo_action(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    undo::undo_one(&conn, &id).map_err(to_str)
}

#[tauri::command]
pub fn undo_actions(state: tauri::State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    undo::undo_many(&conn, &ids).map_err(to_str)
}

// ------------------------------------------------------------------- settings

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.lock().map_err(lock_err)?.clone())
}

#[derive(Deserialize)]
pub struct SettingsUpdate {
    pub model: Option<String>,
    pub ollama_url: Option<String>,
    pub retention_days: Option<u64>,
    pub allow_execute: Option<bool>,
    pub allow_web: Option<bool>,
    pub managed_roots: Option<Vec<String>>,
    pub user_name: Option<String>,
    pub persona: Option<String>,
    pub about_you: Option<String>,
    pub theme: Option<String>,
    pub onboarded: Option<bool>,
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    update: SettingsUpdate,
) -> Result<Config, String> {
    // Apply to the in-memory config and take a snapshot (config lock released here).
    let new_cfg = {
        let mut cfg = state.config.lock().map_err(lock_err)?;
        if let Some(v) = update.model {
            cfg.model = v;
        }
        if let Some(v) = update.ollama_url {
            cfg.ollama_url = v;
        }
        if let Some(v) = update.retention_days {
            cfg.retention_days = v;
        }
        if let Some(v) = update.allow_execute {
            cfg.allow_execute = v;
        }
        if let Some(v) = update.allow_web {
            cfg.allow_web = v;
        }
        if let Some(v) = update.managed_roots {
            cfg.managed_roots = v;
        }
        if let Some(v) = update.user_name {
            cfg.user_name = v;
        }
        if let Some(v) = update.persona {
            cfg.persona = v;
        }
        if let Some(v) = update.about_you {
            cfg.about_you = v;
        }
        if let Some(v) = update.theme {
            cfg.theme = v;
        }
        if let Some(v) = update.onboarded {
            cfg.onboarded = v;
        }
        cfg.clone()
    };

    {
        let conn = state.db.lock().map_err(lock_err)?;
        state::persist_config(&conn, &new_cfg).map_err(to_str)?;
    }
    // Rebuild the Ollama client in case the URL changed.
    *state.ollama.lock().map_err(lock_err)? = OllamaClient::new(&new_cfg.ollama_url);
    Ok(new_cfg)
}

// --------------------------------------------------------------------- ollama

#[derive(Serialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub version: String,
    pub url: String,
}

#[tauri::command]
pub async fn ollama_status(state: tauri::State<'_, AppState>) -> Result<OllamaStatus, String> {
    let ollama = state.ollama.lock().map_err(lock_err)?.clone();
    let url = ollama.base().to_string();
    match ollama.version().await {
        Ok(version) => Ok(OllamaStatus {
            running: true,
            version,
            url,
        }),
        Err(_) => Ok(OllamaStatus {
            running: false,
            version: String::new(),
            url,
        }),
    }
}

#[tauri::command]
pub async fn list_models(state: tauri::State<'_, AppState>) -> Result<Vec<ModelStatus>, String> {
    let ollama = state.ollama.lock().map_err(lock_err)?.clone();
    ollama.catalog().await.map_err(|e| e.to_string())
}

/// Download a model via the Ollama CLI (`ollama pull <name>`). Can take a while.
#[tauri::command]
pub async fn pull_model(name: String) -> Result<(), String> {
    let out = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new("ollama")
            .arg("pull")
            .arg(&name)
            .output()
    })
    .await
    .map_err(to_str)?
    .map_err(|e| format!("couldn't run `ollama` (is it installed?): {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Open an http(s) link in the user's default browser (e.g. the Discord invite).
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http(s) links can be opened".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(to_str)?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(to_str)?;
    }
    Ok(())
}

/// Detect the PC's capability and recommend a suitable model.
#[tauri::command]
pub async fn system_info() -> Result<crate::system::SystemInfo, String> {
    tauri::async_runtime::spawn_blocking(crate::system::gather)
        .await
        .map_err(to_str)
}

// ----------------------------------------------------------------- files view

#[tauri::command]
pub fn fs_list_dir(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<Vec<FileEntry>, String> {
    let root = match path {
        Some(p) => PathBuf::from(p),
        None => dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
    };
    let _ = &state; // not needed, but keeps a uniform signature
    fsops::list_dir(&root).map_err(to_str)
}

#[tauri::command]
pub async fn fs_analyze(path: String) -> Result<FolderAnalysis, String> {
    tauri::async_runtime::spawn_blocking(move || fsops::analyze_folder(Path::new(&path)))
        .await
        .map_err(to_str)?
        .map_err(to_str)
}

#[tauri::command]
pub async fn fs_storage(path: Option<String>) -> Result<StorageStats, String> {
    let root = path
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    tauri::async_runtime::spawn_blocking(move || fsops::storage_stats(&root))
        .await
        .map_err(to_str)?
        .map_err(to_str)
}

// ------------------------------------------------------------- plugins / memory

#[tauri::command]
pub fn plugins_roadmap() -> Vec<PluginInfo> {
    PluginRegistry::roadmap()
}

#[tauri::command]
pub fn frequent_locations(state: tauri::State<'_, AppState>) -> Result<Vec<Location>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    Ok(memory::frequent_locations(&conn, 12))
}

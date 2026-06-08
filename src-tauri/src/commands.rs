//! Tauri command handlers — the thin bridge between the React UI and the trusted
//! core. Commands never bypass the validator: mutations always go
//! intent -> planner -> `safety::validate` -> pending map -> `executor` (on approve).

use crate::ai::{ModelStatus, OllamaClient};
use crate::executor::{self, ExecutionReport};
use crate::models::{
    Action, ActionLogEntry, AiPlan, Conversation, FileEntry, FolderAnalysis, GenStats, Location,
    MemoryItem, Message, Operation, QueryResult, StorageStats, TrashItem,
};
use crate::plugin::{PluginInfo, PluginRegistry};
use crate::safety::ValidatedPlan;
use crate::state::{self, AppState, Config};
use crate::{ai, db, fsops, intent, memory, planner, safety, trash, undo};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::Emitter;

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

/// A streamed prose delta pushed to the UI during generation.
#[derive(Clone, Serialize)]
struct TokenEvent {
    id: String,
    delta: String,
}

/// Tells the UI to clear the streamed text (before a corrective retry).
#[derive(Clone, Serialize)]
struct ResetEvent {
    id: String,
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

/// Turn a raw Ollama/transport error into a message the user can actually act on.
fn friendly_ollama_error(model: &str, err: &str) -> String {
    let e = err.to_lowercase();
    if e.contains("reach")
        || e.contains("connect")
        || e.contains("timed out")
        || e.contains("timeout")
        || e.contains("os error")
        || e.contains("dns")
    {
        "I can't reach Ollama right now 🪨 — make sure it's running (open a terminal and run \
         `ollama serve`), then try again. Pebble needs Ollama running locally to think."
            .to_string()
    } else if e.contains("not found") || e.contains("pull") || e.contains("no such model") {
        format!(
            "The model \"{model}\" isn't installed yet. Open Settings → AI Model to download it \
             (or run `ollama pull {model}`), then try again."
        )
    } else {
        format!("Something went wrong talking to Ollama: {err}")
    }
}

/// If an action belongs to a disabled extension, the message to show instead.
fn disabled_extension_message(
    action: &Action,
    ext_content: bool,
    ext_ocr: bool,
    ext_dedupe: bool,
) -> Option<String> {
    match action {
        Action::SearchContent { .. } if !ext_content => Some(
            "Content Search is off. Turn it on in Settings → Extensions so I can search inside files."
                .into(),
        ),
        Action::ReadImageText { .. } if !ext_ocr => Some(
            "OCR is off. Turn it on in Settings → Extensions so I can read text from images.".into(),
        ),
        Action::CleanDuplicates { .. } if !ext_dedupe => {
            Some("Duplicate Cleaner is off. Turn it on in Settings → Extensions first.".into())
        }
        _ => None,
    }
}

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    content: String,
    mode: Option<String>,
) -> Result<ChatResponse, String> {
    let planning = mode.as_deref() == Some("plan");
    let just_chat = mode.as_deref() == Some("chat");
    let conversational = planning || just_chat; // no actions in chat or plan mode
    let iso = chrono::Local::now().format("%Y-%m-%d").to_string();
    let human = chrono::Local::now().format("%A, %B %e, %Y").to_string();

    // 1) Gather context under locks, then release before any await.
    let (messages, model, ollama, safety_cfg, allow_web, allow_weather, ext_content, ext_ocr, ext_dedupe) = {
        let conn = state.db.lock().map_err(lock_err)?;
        let cfg = state.config.lock().map_err(lock_err)?;
        db::insert_message(&conn, &conversation_id, "user", &content, None).map_err(to_str)?;
        let recall = if cfg.allow_memory {
            memory::recall_block(&conn, &cfg.user_name, &iso)
        } else {
            String::new()
        };
        let system = if just_chat {
            ai::chat_prompt(&cfg.user_name, &cfg.persona, &cfg.about_you, cfg.adapt_tone, &human, &recall)
        } else if planning {
            ai::planning_prompt(&cfg.user_name, &cfg.persona, &cfg.about_you, cfg.adapt_tone, &human, &recall)
        } else {
            ai::system_prompt(
                &state.platform.known_folders,
                &state.platform.common_locations,
                &cfg.trash_root,
                &cfg.user_name,
                &cfg.persona,
                &cfg.about_you,
                cfg.adapt_tone,
                &human,
                &recall,
                cfg.allow_web,
                cfg.allow_weather,
                cfg.ext_content_search,
                cfg.ext_ocr,
                cfg.ext_dedupe,
            )
        };
        let messages =
            memory::conversation_messages(&conn, &conversation_id, system, 8).map_err(to_str)?;
        let model = cfg.model.clone();
        let ollama = state.ollama.lock().map_err(lock_err)?.clone();
        let safety_cfg = state.safety_config(&cfg);
        (
            messages,
            model,
            ollama,
            safety_cfg,
            cfg.allow_web,
            cfg.allow_weather,
            cfg.ext_content_search,
            cfg.ext_ocr,
            cfg.ext_dedupe,
        )
    };

    // 2) Ask the model, streaming tokens to the UI as they arrive. Plan mode =
    //    plain prose; Do mode = structured JSON (we surface only the "message"
    //    field as it streams; actions are parsed once the object is complete).
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();
    let emit_app = app.clone();
    let emit_cid = conversation_id.clone();
    let outcome = match ollama
        .chat_stream(&model, &messages, !conversational, cancel.clone(), move |delta| {
            let _ = emit_app.emit(
                "chat:token",
                TokenEvent {
                    id: emit_cid.clone(),
                    delta: delta.to_string(),
                },
            );
        })
        .await
    {
        Ok(o) => o,
        Err(e) => return Err(friendly_ollama_error(&model, &e.to_string())),
    };
    let cancelled = cancel.load(Ordering::Relaxed);

    // ---- Conversational modes (Chat / Plan): just talk, never execute. ----
    if conversational {
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
    // If the user hit Stop mid-stream the JSON is incomplete, so use the prose we
    // streamed rather than trying to parse a half-finished object.
    let mut ai_plan = if cancelled {
        AiPlan {
            message: ai::ollama::extract_partial_message(&outcome.content)
                .unwrap_or_else(|| outcome.content.trim().to_string()),
            actions: Vec::new(),
        }
    } else {
        intent::parse(&outcome.content)
    };
    let mut stats_outcome = outcome;

    // Corrective retry: if Pebble claimed it's "done" but produced no actions,
    // nudge it once to actually act or ask — words alone change nothing. Skipped
    // if the user cancelled.
    if !cancelled && ai_plan.actions.is_empty() && looks_like_false_completion(&ai_plan.message) {
        let _ = app.emit(
            "chat:reset",
            ResetEvent {
                id: conversation_id.clone(),
            },
        );
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
        let r_app = app.clone();
        let r_cid = conversation_id.clone();
        if let Ok(o2) = ollama
            .chat_stream(&model, &retry, true, cancel.clone(), move |delta| {
                let _ = r_app.emit(
                    "chat:token",
                    TokenEvent {
                        id: r_cid.clone(),
                        delta: delta.to_string(),
                    },
                );
            })
            .await
        {
            ai_plan = if cancel.load(Ordering::Relaxed) {
                AiPlan {
                    message: ai::ollama::extract_partial_message(&o2.content)
                        .unwrap_or_else(|| o2.content.trim().to_string()),
                    actions: Vec::new(),
                }
            } else {
                intent::parse(&o2.content)
            };
            stats_outcome = o2;
        }
    }

    // 3) Route actions (heavy filesystem reads run off the async thread).
    let mut query_results: Vec<QueryResult> = Vec::new();
    let mut ops: Vec<Operation> = Vec::new();
    for action in &ai_plan.actions {
        if let Some(msg) = disabled_extension_message(action, ext_content, ext_ocr, ext_dedupe) {
            query_results.push(QueryResult::Error { message: msg });
            continue;
        }
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
            planner::Class::Weather => {
                if let Action::GetWeather { location } = action {
                    if !allow_weather {
                        query_results.push(QueryResult::Error {
                            message: "Weather is off. Turn it on in Settings → Abilities (it uses \
                                      the internet)."
                                .into(),
                        });
                    } else {
                        match crate::weather::fetch(location.as_deref()).await {
                            Ok(info) => query_results.push(QueryResult::Weather { info }),
                            Err(e) => query_results.push(QueryResult::Error {
                                message: format!("Couldn't get the weather: {e}"),
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

/// Ask an in-flight streaming generation to stop (the Stop button).
#[tauri::command]
pub fn cancel_generation(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// After a turn, let Pebble decide what (if anything) is worth remembering about
/// the user, and store it. No-op unless memory is enabled. Best-effort, async.
#[tauri::command]
pub async fn extract_memories(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<usize, String> {
    let (model, ollama, name, transcript) = {
        let conn = state.db.lock().map_err(lock_err)?;
        let cfg = state.config.lock().map_err(lock_err)?;
        if !cfg.allow_memory {
            return Ok(0);
        }
        let msgs = db::list_messages(&conn, &conversation_id).map_err(to_str)?;
        let recent: Vec<String> = msgs
            .iter()
            .rev()
            .take(6)
            .rev()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect();
        if recent.is_empty() {
            return Ok(0);
        }
        let name = if cfg.user_name.trim().is_empty() {
            "the user".to_string()
        } else {
            cfg.user_name.trim().to_string()
        };
        let model = cfg.model.clone();
        let ollama = state.ollama.lock().map_err(lock_err)?.clone();
        (model, ollama, name, recent.join("\n"))
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let prompt = format!(
        "You quietly keep a few long-term memories about {name} so you can be a better friend later. \
         From the conversation below, extract ONLY durable, personal things worth remembering about them \
         as a person — their life, preferences, relationships, plans, feelings, or important dated events. \
         IGNORE file/computer tasks, requests, and small talk. If they mention an event with a date, \
         include it as YYYY-MM-DD (today is {today}; resolve things like 'next Friday' or 'July 1st'). \
         Reply with ONLY JSON: {{\"memories\":[{{\"content\":\"short note in third person\",\"date\":\"YYYY-MM-DD or empty\"}}]}}. \
         Use an empty list if there's nothing worth keeping.\n\nConversation:\n{transcript}"
    );
    let messages = vec![ai::ChatMessage {
        role: "user".into(),
        content: prompt,
    }];
    let out = match ollama.chat(&model, &messages, true).await {
        Ok(o) => o,
        Err(_) => return Ok(0),
    };
    let v: serde_json::Value = match serde_json::from_str(out.content.trim()) {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };
    let arr = match v.get("memories").and_then(|m| m.as_array()) {
        Some(a) => a.clone(),
        None => return Ok(0),
    };

    let conn = state.db.lock().map_err(lock_err)?;
    let mut stored = 0usize;
    for m in arr.iter().take(8) {
        let content = m
            .get("content")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if content.is_empty() || db::memory_exists(&conn, &content) {
            continue;
        }
        let date = m
            .get("date")
            .and_then(|s| s.as_str())
            .map(|s| s.trim())
            .filter(|s| s.len() == 10);
        let kind = if date.is_some() { "event" } else { "fact" };
        if db::insert_memory(&conn, kind, &content, date).is_ok() {
            stored += 1;
        }
    }
    Ok(stored)
}

/// Pebble proactively asks the user one warm question (uses memory + the date).
/// Persists it as an assistant message and returns the question.
#[tauri::command]
pub async fn pebble_question(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<String, String> {
    let (model, ollama, prompt) = {
        let conn = state.db.lock().map_err(lock_err)?;
        let cfg = state.config.lock().map_err(lock_err)?;
        let iso = chrono::Local::now().format("%Y-%m-%d").to_string();
        let human = chrono::Local::now().format("%A, %B %e, %Y").to_string();
        let recall = if cfg.allow_memory {
            memory::recall_block(&conn, &cfg.user_name, &iso)
        } else {
            String::new()
        };
        let name = if cfg.user_name.trim().is_empty() {
            "your friend".to_string()
        } else {
            cfg.user_name.trim().to_string()
        };
        let about = if cfg.about_you.trim().is_empty() {
            String::new()
        } else {
            format!("\nThings they told you: {}", cfg.about_you.trim())
        };
        let prompt = format!(
            "You are Pebble, {name}'s friend. Ask {name} ONE short, warm, genuine question — like a friend \
             checking in. If you remember something relevant (especially an upcoming or recent event), follow \
             up on THAT (e.g. \"how'd the math test go?\"). Otherwise ask a light get-to-know-you question. \
             One or two sentences, casual, no preamble. Today is {human}.{about}{recall}\n\nReply with ONLY the question.",
        );
        let model = cfg.model.clone();
        let ollama = state.ollama.lock().map_err(lock_err)?.clone();
        (model, ollama, prompt)
    };
    let messages = vec![ai::ChatMessage {
        role: "user".into(),
        content: prompt,
    }];
    let q = match ollama.chat(&model, &messages, false).await {
        Ok(o) => o.content.trim().to_string(),
        Err(e) => return Err(friendly_ollama_error(&model, &e.to_string())),
    };
    let q = if q.is_empty() {
        "Hey — how's your day going? 🤍".to_string()
    } else {
        q
    };
    {
        let conn = state.db.lock().map_err(lock_err)?;
        db::insert_message(&conn, &conversation_id, "assistant", &q, None).map_err(to_str)?;
    }
    Ok(q)
}

#[tauri::command]
pub fn list_memory(state: tauri::State<'_, AppState>) -> Result<Vec<MemoryItem>, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::list_memory(&conn).map_err(to_str)
}

#[tauri::command]
pub fn delete_memory(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::delete_memory(&conn, &id).map_err(to_str)
}

#[tauri::command]
pub fn clear_memory(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(lock_err)?;
    db::clear_memory(&conn).map_err(to_str)
}

/// Read text out of an image (OCR). Gated by the OCR extension. Persists the
/// exchange and returns the extracted text.
#[tauri::command]
pub async fn read_image(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    path: String,
) -> Result<String, String> {
    {
        let cfg = state.config.lock().map_err(lock_err)?;
        if !cfg.ext_ocr {
            return Err("To read images, turn on OCR in Settings → Extensions first.".into());
        }
    }
    let p = path.clone();
    let text = tauri::async_runtime::spawn_blocking(move || crate::ocr::image_text(&p))
        .await
        .map_err(to_str)?
        .map_err(|e| e.to_string())?;
    let fname = Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    {
        let conn = state.db.lock().map_err(lock_err)?;
        db::insert_message(&conn, &conversation_id, "user", &format!("📷 {fname}"), None)
            .map_err(to_str)?;
        db::insert_message(&conn, &conversation_id, "assistant", &text, None).map_err(to_str)?;
    }
    Ok(text)
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
    let safety_cfg = state.safety_config(&cfg);
    let conn = state.db.lock().map_err(lock_err)?;
    Ok(executor::execute(&plan, &conn, &trash_cfg, &safety_cfg))
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
    pub ext_content_search: Option<bool>,
    pub ext_ocr: Option<bool>,
    pub ext_dedupe: Option<bool>,
    pub allow_weather: Option<bool>,
    pub allow_memory: Option<bool>,
    pub adapt_tone: Option<bool>,
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
        if let Some(v) = update.ext_content_search {
            cfg.ext_content_search = v;
        }
        if let Some(v) = update.ext_ocr {
            cfg.ext_ocr = v;
        }
        if let Some(v) = update.ext_dedupe {
            cfg.ext_dedupe = v;
        }
        if let Some(v) = update.allow_weather {
            cfg.allow_weather = v;
        }
        if let Some(v) = update.allow_memory {
            cfg.allow_memory = v;
        }
        if let Some(v) = update.adapt_tone {
            cfg.adapt_tone = v;
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

//! Local AI Computer Assistant — application entry point and module graph.
//!
//! Architecture (data flows left to right; the safety boundary is hard):
//!
//!   UI ─▶ ai(ollama) ─▶ intent ─▶ planner ─▶ ⛨ safety ⛨ ─▶ (confirm) ─▶ executor
//!                                                                          │
//!                                                          trash · undo · db · memory
//!
//! The `safety` module is independent of `ai`/`intent`/`planner`; it is the only
//! producer of `ValidatedPlan`, which is the only thing `executor` will run.

mod ai;
mod commands;
mod db;
mod executor;
mod fsops;
mod fsutil;
mod intent;
mod memory;
mod models;
mod planner;
mod platform;
mod plugin;
mod safety;
mod state;
mod system;
mod trash;
mod undo;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new().expect("failed to initialize application state");

    // Best-effort: purge expired trash on startup (honors retention setting).
    if let Ok(conn) = app_state.db.lock() {
        let _ = trash::cleanup(&conn);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::ensure_conversation,
            commands::new_conversation,
            commands::list_conversations,
            commands::conversation_messages,
            commands::rename_conversation,
            commands::auto_title,
            commands::delete_conversation,
            commands::approve_plan,
            commands::reject_plan,
            commands::propose_delete,
            commands::propose_move,
            commands::list_trash,
            commands::restore_trash,
            commands::delete_trash_item,
            commands::empty_trash,
            commands::cleanup_trash,
            commands::list_action_log,
            commands::undo_last,
            commands::undo_action,
            commands::undo_actions,
            commands::get_config,
            commands::update_settings,
            commands::ollama_status,
            commands::list_models,
            commands::pull_model,
            commands::open_url,
            commands::system_info,
            commands::fs_list_dir,
            commands::fs_analyze,
            commands::fs_storage,
            commands::plugins_roadmap,
            commands::frequent_locations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

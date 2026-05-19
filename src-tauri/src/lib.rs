mod ai;
mod audio;
mod commands;
mod db;
mod error;
mod mcp_server;
mod model;
mod state;
mod transcription;

use ai::{
    claude::ClaudeProvider,
    gemini::GeminiProvider,
    harness::AgentHarness,
    mcp::{discovery::load_claude_desktop_configs, McpManager},
    AiProvider,
};
use commands::{
    chat, mcp_server as mcp_server_cmd, notes, search, sessions, settings, tags, tasks,
};
use state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("darric_lib=debug"))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = db::open().map_err(|e| e.to_string())?;

            // Build AI harness from saved settings
            let mcp_configs = load_claude_desktop_configs();
            let mcp = Arc::new(McpManager::new(mcp_configs));
            let provider_name = db::get_setting(&conn, "ai.provider");
            let claude_key = db::get_setting(&conn, "ai.claude.api_key");
            let gemini_key = db::get_setting(&conn, "ai.gemini.api_key");
            let provider: Option<AiProvider> = match provider_name.as_deref() {
                Some("gemini") => gemini_key.map(|k| AiProvider::Gemini(GeminiProvider::new(k))),
                _ => claude_key.map(|k| AiProvider::Claude(ClaudeProvider::new(k))),
            };
            let harness = provider.map(|p| AgentHarness::new(p, mcp.clone()));
            let history = db::load_chat_history(&conn);

            let mcp_server_enabled =
                db::get_setting(&conn, "mcp_server.enabled").as_deref() != Some("false");
            let mcp_server_port = db::get_setting(&conn, "mcp_server.port")
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(27842);

            let state = AppState::new(conn, harness, history, mcp);

            // Pre-load whisper in background so the first session starts instantly
            let transcriber_slot = state.transcriber.clone();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match model::ensure_model(&handle).await {
                    Ok(path) => {
                        log::info!("[startup] loading whisper model into memory…");
                        let path_str = path.to_string_lossy().into_owned();
                        match tauri::async_runtime::spawn_blocking(move || {
                            transcription::Transcriber::new(&path_str)
                        })
                        .await
                        {
                            Ok(Ok(t)) => {
                                *transcriber_slot.lock().unwrap() = Some(Arc::new(t));
                                log::info!("[startup] whisper ready");
                                handle.emit("model_ready", ()).ok();
                            }
                            Ok(Err(e)) => log::error!("[startup] whisper load failed: {e}"),
                            Err(e) => log::error!("[startup] spawn_blocking failed: {e}"),
                        }
                    }
                    Err(e) => log::error!("[startup] model download failed: {e}"),
                }
            });

            let mcp_server_slot = state.mcp_server.clone();
            let mcp_server_db = state.db.clone();
            if mcp_server_enabled {
                tauri::async_runtime::spawn(async move {
                    match mcp_server::spawn(mcp_server_db, mcp_server_port).await {
                        Ok(srv) => {
                            *mcp_server_slot.lock().unwrap() = Some(srv);
                        }
                        Err(e) => log::error!("[mcp_server] startup failed: {e}"),
                    }
                });
            } else {
                log::info!("[mcp_server] disabled via settings");
            }

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sessions::start_session,
            sessions::stop_session,
            sessions::list_sessions,
            sessions::get_session_transcript,
            sessions::delete_session,
            sessions::update_session,
            sessions::update_session_notes,
            sessions::resume_session,
            settings::save_setting,
            settings::get_setting,
            settings::list_mcp_servers,
            notes::list_notes,
            notes::get_note,
            notes::create_note,
            notes::update_note,
            notes::delete_note,
            tasks::list_tasks,
            tasks::create_task,
            tasks::update_task,
            tasks::delete_task,
            chat::send_chat_message,
            chat::get_chat_history,
            chat::clear_chat_history,
            search::search_all,
            tags::list_tags,
            tags::add_tag_to_session,
            tags::remove_tag_from_session,
            tags::add_tag_to_note,
            tags::remove_tag_from_note,
            tags::add_tag_to_task,
            tags::remove_tag_from_task,
            mcp_server_cmd::mcp_server_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

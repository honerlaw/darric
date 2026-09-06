mod audio;
mod commands;
mod db;
mod error;
mod mcp_server;
mod model;
mod state;
mod transcription;

use commands::{devices, mcp_server as mcp_commands, model as model_commands, sessions, settings};
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("darric_lib=debug"))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = db::open().map_err(|e| e.to_string())?;
            let disabled = devices::load_disabled(&conn);
            let state = AppState::new(conn, disabled);

            // Pre-load whisper in the background so the first session starts
            // instantly. This goes through the same single-flight loader as the
            // session path, so a recording started while this is still
            // downloading waits for it rather than racing it.
            let transcriber_slot = state.transcriber.clone();
            let db = Arc::clone(&state.db);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match transcription::loader::get_or_load(&handle, &transcriber_slot, &db).await {
                    Ok(_) => log::info!("[startup] whisper ready"),
                    Err(e) => log::error!("[startup] whisper unavailable: {e}"),
                }
            });

            app.manage(state);
            start_mcp_server(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sessions::start_session,
            sessions::stop_session,
            sessions::list_sessions,
            sessions::get_session_transcript,
            sessions::delete_session,
            sessions::update_session,
            sessions::resume_session,
            settings::save_setting,
            settings::get_setting,
            mcp_commands::mcp_server_status,
            devices::list_capture_devices,
            devices::set_device_enabled,
            devices::capture_drop_count,
            model_commands::model_download_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Bring up the loopback MCP server and record the outcome in `AppState`.
///
/// Runs after `app.manage`, because the server's `status` tool reads the
/// engine through the app handle. A bind failure — the port held by another
/// process — is recorded for the header chip and the app carries on without
/// the endpoint; recording does not depend on it.
fn start_mcp_server(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let outcome = match mcp_server::bind(mcp_server::DEFAULT_PORT) {
        Err(e) => {
            log::error!("[mcp_server] not started, port busy: {e}");
            mcp_server::McpServerState::PortBusy(e.to_string())
        }
        Ok(listener) => {
            let served = db::open_read_only(&db::path()).and_then(|reader| {
                let reader = Arc::new(state::DbConn(std::sync::Mutex::new(reader)));
                let live = Arc::new(mcp_commands::AppLiveStatus(app.clone()));
                mcp_server::serve(listener, reader, live)
            });
            match served {
                Ok((handle, future)) => {
                    tauri::async_runtime::spawn(future);
                    mcp_server::McpServerState::Listening(handle)
                }
                Err(e) => {
                    log::error!("[mcp_server] not started: {e}");
                    mcp_server::McpServerState::Failed(e.to_string())
                }
            }
        }
    };
    *state
        .mcp_server
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = outcome;
}

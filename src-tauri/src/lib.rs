mod audio;
mod commands;
mod db;
mod error;
mod model;
mod state;
mod transcription;

use commands::{devices, model as model_commands, sessions, settings};
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
            devices::list_capture_devices,
            devices::set_device_enabled,
            devices::capture_drop_count,
            model_commands::model_download_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

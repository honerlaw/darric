mod audio;
mod commands;
mod db;
mod error;
mod model;
mod state;
mod transcription;

use commands::{devices, sessions, settings};
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
            let disabled = devices::load_disabled(&conn);
            let state = AppState::new(conn, disabled);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

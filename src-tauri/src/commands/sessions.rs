use crate::{
    audio::{device, CaptureEngine},
    error::{AppError, Result},
    state::AppState,
    transcription::Transcriber,
};
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// Whisper worker threads draining the shared segment queue.
///
/// Measured, not guessed — `transcription::bench::pool_sizing_measurement` runs
/// four 8-second segments serially and then concurrently against one shared
/// `WhisperContext`. On this machine (Apple Silicon, Metal, `small.en`) the
/// result was 1.745 s serial vs 1.537 s parallel: a **1.14x speedup from 4x the
/// threads**. Inference is effectively serialised on the GPU, and the small gain
/// is CPU-side pre/post-processing overlapping.
///
/// So the pool is not a throughput device. Two workers recover that ~14% and
/// keep one thread ready while the other is in CPU-side work; more would add
/// per-worker state memory for nothing. What actually protects the recording
/// when transcription falls behind is the queue's drop-oldest policy, exactly as
/// the proposal anticipated.
///
/// Caveat: measured with `small.en` because that is what was downloaded.
/// `large-v3-turbo` is a larger model; the serialisation conclusion is a
/// property of the GPU queue and should hold, but the absolute timings will not.
const WHISPER_WORKERS: usize = 2;

async fn load_transcriber(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Option<Arc<Transcriber>> {
    let cached = state
        .transcriber
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if cached.is_some() {
        log::info!("[session] using pre-loaded transcriber");
        return cached;
    }
    log::info!("[session] transcriber not ready yet — loading now");

    let custom = {
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        db.query_row(
            "SELECT value FROM settings WHERE key = 'whisper_model_path'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.is_empty())
    };

    let path = match custom {
        Some(p) => Ok(std::path::PathBuf::from(p)),
        None => crate::model::ensure_model(app).await,
    };

    match path {
        Ok(p) => {
            let path_str = p.to_string_lossy().into_owned();
            match tokio::task::spawn_blocking(move || Transcriber::new(&path_str)).await {
                Ok(Ok(t)) => {
                    let t = Arc::new(t);
                    *state
                        .transcriber
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(t.clone());
                    app.emit("model_ready", ()).ok();
                    Some(t)
                }
                Ok(Err(e)) => {
                    log::error!("[session] transcriber load failed: {e}");
                    None
                }
                Err(e) => {
                    log::error!("[session] spawn_blocking failed: {e}");
                    None
                }
            }
        }
        Err(e) => {
            log::error!("[session] model unavailable: {e}");
            None
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub topic: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub recorded_minutes: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct TranscriptLine {
    pub id: String,
    pub session_id: String,
    pub device_id: String,
    pub device_name: String,
    pub direction: String,
    pub content: String,
    pub recorded_at: String,
}

const SESSION_SELECT: &str = "SELECT s.id, s.topic, s.started_at, s.ended_at, s.created_at,
   COALESCE(CAST(SUM(
     (julianday(COALESCE(seg.ended_at, datetime('now'))) - julianday(seg.started_at)) * 1440
   ) AS INTEGER), 0) as recorded_minutes
 FROM sessions s
 LEFT JOIN recording_segments seg ON seg.session_id = s.id";

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        topic: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        created_at: row.get(4)?,
        recorded_minutes: row.get(5)?,
    })
}

/// Devices to capture: everything discovered, minus the user's exceptions.
fn devices_to_capture(state: &tauri::State<'_, AppState>) -> Vec<device::CaptureDevice> {
    let disabled = state
        .disabled_devices
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let mut all = device::list_input_devices(&state.exclusions);
    all.extend(device::list_output_devices(&state.exclusions));
    all.into_iter()
        .filter(|d| !disabled.contains(&d.id))
        .collect()
}

async fn begin_capture(
    app: AppHandle,
    state: &tauri::State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    let transcriber = load_transcriber(&app, state).await;
    let devices = devices_to_capture(state);

    if devices.is_empty() {
        return Err(AppError::Audio(
            "no capture devices are enabled — enable at least one to record".to_string(),
        ));
    }
    log::info!(
        "[session] capturing {} device(s): {}",
        devices.len(),
        devices
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let engine = CaptureEngine::start(
        session_id,
        devices,
        transcriber,
        WHISPER_WORKERS,
        &app,
        &state.db,
        &state.exclusions,
    )?;
    *state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(engine);
    Ok(())
}

#[tauri::command]
pub async fn start_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    topic: Option<String>,
) -> Result<String> {
    if state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_some()
    {
        return Err(AppError::SessionActive);
    }

    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    {
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        db.execute(
            "INSERT INTO sessions(id, topic, started_at, created_at) VALUES(?1, ?2, ?3, ?3)",
            rusqlite::params![session_id, topic, now],
        )?;
        db.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), session_id, now],
        )?;
    }

    begin_capture(app, &state, session_id.clone()).await?;
    Ok(session_id)
}

#[tauri::command]
pub async fn resume_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<String> {
    if state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_some()
    {
        return Err(AppError::SessionActive);
    }

    {
        let now = Utc::now().to_rfc3339();
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        db.execute(
            "UPDATE sessions SET ended_at = NULL WHERE id = ?1",
            rusqlite::params![id],
        )?;
        db.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), id, now],
        )?;
    }

    begin_capture(app, &state, id.clone()).await?;
    Ok(id)
}

#[tauri::command]
pub async fn stop_session(state: tauri::State<'_, AppState>) -> Result<()> {
    let engine = state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    let Some(engine) = engine else {
        return Err(AppError::NoSession);
    };

    let session_id = engine.session_id().to_string();

    // `stop` joins the capture threads and then waits for the whisper workers to
    // drain the queue — up to several seconds with a few devices, since
    // inference serialises on the GPU. Doing that inline would block a Tokio
    // worker thread and stall every other command scheduled onto it.
    tokio::task::spawn_blocking(move || engine.stop())
        .await
        .map_err(|e| AppError::Audio(format!("stopping the capture engine failed: {e}")))?;

    let now = Utc::now().to_rfc3339();
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    db.execute(
        "UPDATE recording_segments SET ended_at = ?1 WHERE session_id = ?2 AND ended_at IS NULL",
        rusqlite::params![now, session_id],
    )?;
    db.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        rusqlite::params![now, session_id],
    )?;
    Ok(())
}

#[tauri::command]
pub async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<Session>> {
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sql = format!("{SESSION_SELECT} GROUP BY s.id ORDER BY s.started_at DESC");
    let mut stmt = db.prepare(&sql)?;
    let sessions = stmt
        .query_map([], map_session)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(sessions)
}

#[tauri::command]
pub async fn delete_session(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    db.execute(
        "DELETE FROM transcript_lines WHERE session_id = ?1",
        rusqlite::params![id],
    )?;
    db.execute(
        "DELETE FROM recording_segments WHERE session_id = ?1",
        rusqlite::params![id],
    )?;
    db.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

#[tauri::command]
pub async fn update_session(
    state: tauri::State<'_, AppState>,
    id: String,
    topic: Option<String>,
) -> Result<Session> {
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    db.execute(
        "UPDATE sessions SET topic = ?1 WHERE id = ?2",
        rusqlite::params![topic, id],
    )?;
    let sql = format!("{SESSION_SELECT} WHERE s.id = ?1 GROUP BY s.id");
    let mut stmt = db.prepare(&sql)?;
    let session = stmt.query_row(rusqlite::params![id], map_session)?;
    Ok(session)
}

#[tauri::command]
pub async fn get_session_transcript(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<Vec<TranscriptLine>> {
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut stmt = db.prepare(
        "SELECT id, session_id, device_id, device_name, direction, content, recorded_at
         FROM transcript_lines WHERE session_id = ?1 ORDER BY recorded_at ASC",
    )?;
    let lines = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(TranscriptLine {
                id: row.get(0)?,
                session_id: row.get(1)?,
                device_id: row.get(2)?,
                device_name: row.get(3)?,
                direction: {
                    let raw: String = row.get(4)?;
                    if crate::transcription::pool::Direction::parse(&raw).is_none() {
                        log::warn!(
                            "[session] transcript line {} has unknown direction {raw:?}",
                            row.get::<_, String>(0)?
                        );
                    }
                    raw
                },
                content: row.get(5)?,
                recorded_at: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(lines)
}

use crate::{
    audio,
    commands::tags::Tag,
    error::{AppError, Result},
    state::AppState,
    transcription::Transcriber,
};
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

async fn load_transcriber(app: &AppHandle, state: &tauri::State<'_, AppState>) -> Option<Arc<Transcriber>> {
    let cached = state.transcriber.lock().unwrap().clone();
    if cached.is_some() {
        log::info!("[session] using pre-loaded transcriber");
        return cached;
    }
    log::info!("[session] transcriber not ready yet — loading now");
    let path = {
        let custom = {
            let db = state.db.0.lock().unwrap();
            let mut stmt =
                db.prepare("SELECT value FROM settings WHERE key = 'whisper_model_path'").ok()?;
            let mut rows = stmt.query([]).ok()?;
            rows.next().ok()??
                .get::<_, String>(0).ok()
                .filter(|s| !s.is_empty())
        };
        match custom {
            Some(p) => Ok(std::path::PathBuf::from(p)),
            None => crate::model::ensure_model(app).await,
        }
    };
    match path {
        Ok(p) => {
            let path_str = p.to_string_lossy().into_owned();
            match tokio::task::spawn_blocking(move || Transcriber::new(&path_str)).await {
                Ok(Ok(t)) => {
                    let t = Arc::new(t);
                    *state.transcriber.lock().unwrap() = Some(t.clone());
                    app.emit("model_ready", ()).ok();
                    Some(t)
                }
                Ok(Err(e)) => { log::error!("[session] transcriber load failed: {e}"); None }
                Err(e) => { log::error!("[session] spawn_blocking failed: {e}"); None }
            }
        }
        Err(e) => { log::error!("[session] model unavailable: {e}"); None }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub topic: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub notes: String,
    pub recorded_minutes: i64,
    pub tags: Vec<Tag>,
}

#[derive(Debug, serde::Serialize)]
pub struct TranscriptLine {
    pub id: String,
    pub session_id: String,
    pub source: String,
    pub speaker_label: Option<String>,
    pub content: String,
    pub recorded_at: String,
}

#[tauri::command]
pub async fn start_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    topic: Option<String>,
) -> Result<String> {
    {
        let audio = state.audio.lock().unwrap();
        if audio.is_some() {
            return Err(AppError::SessionActive);
        }
    }

    let transcriber = load_transcriber(&app, &state).await;

    state.speaker_tracker.lock().unwrap().reset();

    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let segment_id = Uuid::new_v4().to_string();

    {
        let db = state.db.0.lock().unwrap();
        db.execute(
            "INSERT INTO sessions(id, topic, started_at, created_at) VALUES(?1, ?2, ?3, ?3)",
            rusqlite::params![session_id, topic, now],
        )?;
        db.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![segment_id, session_id, now],
        )?;
    }

    let handle = audio::start_capture(
        session_id.clone(),
        app,
        state.db.clone(),
        transcriber,
        state.speaker_tracker.clone(),
    )?;

    {
        let mut audio = state.audio.lock().unwrap();
        *audio = Some(handle);
    }

    Ok(session_id)
}

#[tauri::command]
pub async fn stop_session(state: tauri::State<'_, AppState>) -> Result<()> {
    let handle = {
        let mut audio = state.audio.lock().unwrap();
        audio.take()
    };

    let Some(handle) = handle else {
        return Err(AppError::NoSession);
    };

    if let Some(tx) = handle.shutdown_tx {
        let _ = tx.send(());
    }

    let now = Utc::now().to_rfc3339();
    let db = state.db.0.lock().unwrap();
    db.execute(
        "UPDATE recording_segments SET ended_at = ?1 WHERE session_id = ?2 AND ended_at IS NULL",
        rusqlite::params![now, handle.session_id],
    )?;
    db.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        rusqlite::params![now, handle.session_id],
    )?;

    Ok(())
}

#[tauri::command]
pub async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<Session>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT s.id, s.topic, s.started_at, s.ended_at, s.created_at, s.notes,
           COALESCE(CAST(SUM(
             (julianday(COALESCE(seg.ended_at, datetime('now'))) - julianday(seg.started_at)) * 1440
           ) AS INTEGER), 0) as recorded_minutes,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM session_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.session_id = s.id
           ), '[]') as tags_json
         FROM sessions s
         LEFT JOIN recording_segments seg ON seg.session_id = s.id
         GROUP BY s.id
         ORDER BY s.started_at DESC",
    )?;
    let sessions = stmt
        .query_map([], |row| {
            let tags_json: String = row.get(7)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(Session {
                id: row.get(0)?,
                topic: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                created_at: row.get(4)?,
                notes: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                recorded_minutes: row.get(6)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(sessions)
}

#[tauri::command]
pub async fn delete_session(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "DELETE FROM transcript_lines WHERE session_id = ?1",
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
    let db = state.db.0.lock().unwrap();
    db.execute(
        "UPDATE sessions SET topic = ?1 WHERE id = ?2",
        rusqlite::params![topic, id],
    )?;
    let mut stmt = db.prepare(
        "SELECT s.id, s.topic, s.started_at, s.ended_at, s.created_at, s.notes,
           COALESCE(CAST(SUM(
             (julianday(COALESCE(seg.ended_at, datetime('now'))) - julianday(seg.started_at)) * 1440
           ) AS INTEGER), 0) as recorded_minutes,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM session_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.session_id = s.id
           ), '[]') as tags_json
         FROM sessions s
         LEFT JOIN recording_segments seg ON seg.session_id = s.id
         WHERE s.id = ?1
         GROUP BY s.id",
    )?;
    let session = stmt.query_row(rusqlite::params![id], |row| {
        let tags_json: String = row.get(7)?;
        let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
        Ok(Session {
            id: row.get(0)?,
            topic: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            created_at: row.get(4)?,
            notes: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            recorded_minutes: row.get(6)?,
            tags,
        })
    })?;
    Ok(session)
}

#[tauri::command]
pub async fn resume_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<String> {
    {
        let audio = state.audio.lock().unwrap();
        if audio.is_some() {
            return Err(AppError::SessionActive);
        }
    }

    let transcriber = load_transcriber(&app, &state).await;

    state.speaker_tracker.lock().unwrap().reset();

    {
        let now = Utc::now().to_rfc3339();
        let segment_id = Uuid::new_v4().to_string();
        let db = state.db.0.lock().unwrap();
        db.execute(
            "UPDATE sessions SET ended_at = NULL WHERE id = ?1",
            rusqlite::params![id],
        )?;
        db.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![segment_id, id, now],
        )?;
    }

    let handle = audio::start_capture(
        id.clone(),
        app,
        state.db.clone(),
        transcriber,
        state.speaker_tracker.clone(),
    )?;
    {
        let mut audio = state.audio.lock().unwrap();
        *audio = Some(handle);
    }

    Ok(id)
}

#[tauri::command]
pub async fn update_session_notes(
    state: tauri::State<'_, AppState>,
    id: String,
    notes: String,
) -> Result<()> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "UPDATE sessions SET notes = ?1 WHERE id = ?2",
        rusqlite::params![notes, id],
    )?;
    Ok(())
}

#[tauri::command]
pub async fn get_session_transcript(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<Vec<TranscriptLine>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT id, session_id, source, speaker_label, content, recorded_at
         FROM transcript_lines WHERE session_id = ?1 ORDER BY recorded_at ASC",
    )?;
    let lines = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(TranscriptLine {
                id: row.get(0)?,
                session_id: row.get(1)?,
                source: row.get(2)?,
                speaker_label: row.get(3)?,
                content: row.get(4)?,
                recorded_at: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(lines)
}

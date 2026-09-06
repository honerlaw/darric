use crate::{
    audio::{device, CaptureEngine},
    error::{AppError, Result},
    state::AppState,
    transcription::loader,
};
use chrono::Utc;
use tauri::AppHandle;
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

pub use crate::db::sessions::{Session, TranscriptLine};

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

/// Erase a session and everything written under it.
///
/// `transcript_lines` is included because `transcript_lines.session_id`
/// references `sessions(id)` with `PRAGMA foreign_keys=ON` and no cascade: a
/// capture that started some devices, wrote a line, and then failed would
/// otherwise make the `sessions` delete fail with a foreign-key error and leave
/// behind exactly the empty session this exists to remove. Run as one
/// transaction so a failure part-way cannot leave a session with no segments.
fn erase_session(conn: &rusqlite::Connection, session_id: &str) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for sql in [
        "DELETE FROM transcript_lines WHERE session_id = ?1",
        "DELETE FROM recording_segments WHERE session_id = ?1",
        "DELETE FROM sessions WHERE id = ?1",
    ] {
        tx.execute(sql, rusqlite::params![session_id])?;
    }
    tx.commit()
}

/// Undo a resume that reopened a session but never started capturing.
fn restore_ended_session(
    conn: &rusqlite::Connection,
    session_id: &str,
    previous_ended_at: Option<&str>,
    segment_id: &str,
) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM recording_segments WHERE id = ?1",
        rusqlite::params![segment_id],
    )?;
    tx.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        rusqlite::params![previous_ended_at, session_id],
    )?;
    tx.commit()
}

async fn begin_capture(
    app: AppHandle,
    state: &tauri::State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    // A session with no transcriber records nothing usable — darric persists
    // transcript lines, not audio — so this is a hard failure rather than a
    // capture that silently transcribes nothing.
    let transcriber = loader::get_or_load(&app, &state.transcriber, &state.db).await?;
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
        &transcriber,
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
    // Held for the whole command: the `engine.is_some()` check below is only
    // meaningful if no other start can interleave between it and the moment
    // capture installs the engine. See `AppState::session_transition`.
    let _transition = state.session_transition.lock().await;

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

    if let Err(e) = begin_capture(app, &state, session_id.clone()).await {
        // The rows above were written before capture was attempted. A start that
        // never captured anything must not leave an empty session behind.
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(cleanup) = erase_session(&db, &session_id) {
            log::error!("[session] could not roll back a failed start: {cleanup}");
        }
        return Err(e);
    }
    Ok(session_id)
}

#[tauri::command]
pub async fn resume_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<String> {
    let _transition = state.session_transition.lock().await;

    if state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_some()
    {
        return Err(AppError::SessionActive);
    }

    // Reopening before capture rather than after, with a rollback on failure.
    // Doing it the other way round means a DB error after capture has started
    // returns `Err` while leaving a live engine installed — the UI would show
    // "not recording" with no Stop button, and every later start would be
    // refused as `SessionActive` until the app was quit.
    let segment_id = Uuid::new_v4().to_string();
    let previous_ended_at: Option<String> = {
        let now = Utc::now().to_rfc3339();
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = db.query_row(
            "SELECT ended_at FROM sessions WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get::<_, Option<String>>(0),
        )?;
        db.execute(
            "UPDATE sessions SET ended_at = NULL WHERE id = ?1",
            rusqlite::params![id],
        )?;
        db.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![segment_id, id, now],
        )?;
        previous
    };

    if let Err(e) = begin_capture(app, &state, id.clone()).await {
        // An open-ended segment is not inert: `recorded_minutes` measures every
        // segment to `COALESCE(ended_at, now)`, so leaving this one behind makes
        // the session's duration climb with the wall clock forever.
        let db = state
            .db
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(cleanup) =
            restore_ended_session(&db, &id, previous_ended_at.as_deref(), &segment_id)
        {
            log::error!("[session] could not roll back a failed resume: {cleanup}");
        }
        return Err(e);
    }

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
    Ok(crate::db::sessions::list_sessions(&db, None, 0)?)
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
    crate::db::sessions::get_session(&db, &id)?
        .ok_or_else(|| AppError::Db(rusqlite::Error::QueryReturnedNoRows))
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
    Ok(crate::db::sessions::transcript_lines(&db, &session_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const SESSION: &str = "11111111-1111-4111-8111-111111111111";
    const TS: &str = "2024-01-01T09:00:00Z";

    fn db() -> Connection {
        crate::db::test_db()
    }

    fn seed_session(conn: &Connection, ended_at: Option<&str>) {
        conn.execute(
            "INSERT INTO sessions(id, topic, started_at, created_at, ended_at)
             VALUES(?1, NULL, ?2, ?2, ?3)",
            rusqlite::params![SESSION, TS, ended_at],
        )
        .expect("seed session");
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .expect("count")
    }

    #[test]
    fn erase_session_removes_transcript_lines_too() {
        // The foreign key from transcript_lines to sessions has no cascade, so
        // deleting the session without them fails and leaves the empty session
        // the rollback exists to remove.
        let conn = db();
        seed_session(&conn, None);
        conn.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES('s1', ?1, ?2)",
            rusqlite::params![SESSION, TS],
        )
        .expect("segment");
        conn.execute(
            "INSERT INTO transcript_lines(
                 id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES('l1', ?1, 'dev', 'Dev', 'input', 'hello', ?2)",
            rusqlite::params![SESSION, TS],
        )
        .expect("line");

        erase_session(&conn, SESSION).expect("rollback succeeds");

        assert_eq!(count(&conn, "sessions"), 0);
        assert_eq!(count(&conn, "recording_segments"), 0);
        assert_eq!(count(&conn, "transcript_lines"), 0);
    }

    #[test]
    fn erase_session_is_a_no_op_for_an_unknown_id() {
        let conn = db();
        seed_session(&conn, None);
        erase_session(&conn, "no-such-session").expect("no-op rollback");
        assert_eq!(count(&conn, "sessions"), 1);
    }

    #[test]
    fn restore_ended_session_puts_a_failed_resume_back_exactly() {
        // A resume that reopened the session and then failed to capture must not
        // leave it open: recorded_minutes measures every segment to
        // COALESCE(ended_at, now), so an open segment grows without bound.
        let conn = db();
        seed_session(&conn, Some(TS));
        conn.execute(
            "UPDATE sessions SET ended_at = NULL WHERE id = ?1",
            rusqlite::params![SESSION],
        )
        .expect("reopen");
        conn.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES('new', ?1, ?2)",
            rusqlite::params![SESSION, TS],
        )
        .expect("segment");

        restore_ended_session(&conn, SESSION, Some(TS), "new").expect("rollback succeeds");

        let ended: Option<String> = conn
            .query_row(
                "SELECT ended_at FROM sessions WHERE id = ?1",
                rusqlite::params![SESSION],
                |r| r.get(0),
            )
            .expect("session still exists");
        assert_eq!(ended.as_deref(), Some(TS), "the session is closed again");
        assert_eq!(
            count(&conn, "recording_segments"),
            0,
            "the new segment is gone"
        );
    }

    #[test]
    fn restore_ended_session_keeps_earlier_segments() {
        let conn = db();
        seed_session(&conn, Some(TS));
        conn.execute(
            "INSERT INTO recording_segments(id, session_id, started_at, ended_at)
             VALUES('old', ?1, ?2, ?2)",
            rusqlite::params![SESSION, TS],
        )
        .expect("old segment");
        conn.execute(
            "INSERT INTO recording_segments(id, session_id, started_at) VALUES('new', ?1, ?2)",
            rusqlite::params![SESSION, TS],
        )
        .expect("new segment");

        restore_ended_session(&conn, SESSION, Some(TS), "new").expect("rollback succeeds");

        assert_eq!(
            count(&conn, "recording_segments"),
            1,
            "only the new one is removed"
        );
    }
}

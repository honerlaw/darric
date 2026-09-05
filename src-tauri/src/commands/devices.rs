use crate::{
    audio::device::{self, CaptureDevice},
    error::Result,
    state::AppState,
};

#[derive(Debug, serde::Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: String,
    pub enabled: bool,
    /// Live capture state, or "idle" when no recording is running.
    pub state: String,
    pub level: f32,
}

/// Every capture device, merged with its live state if a recording is running.
#[tauri::command]
pub async fn list_capture_devices(state: tauri::State<'_, AppState>) -> Result<Vec<DeviceInfo>> {
    let devices = device::list_input_devices();
    let disabled = state
        .disabled_devices
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();

    let live: Vec<_> = state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .map(super::super::audio::CaptureEngine::statuses)
        .unwrap_or_default();

    Ok(devices
        .into_iter()
        .map(|d: CaptureDevice| {
            let status = live.iter().find(|s| s.device.id == d.id);
            DeviceInfo {
                enabled: !disabled.contains(&d.id),
                state: status.map_or("idle", |s| s.state.as_str()).to_string(),
                level: status.map_or(0.0, |s| s.level),
                direction: d.direction.as_str().to_string(),
                id: d.id,
                name: d.name,
            }
        })
        .collect())
}

/// Turn one device on or off for future recordings.
///
/// Takes effect on the next recording rather than mid-session: adding or
/// removing a source from a running engine would leave the transcript with an
/// unexplained gap, and stopping the recording is the honest way to change what
/// is being captured.
#[tauri::command]
pub async fn set_device_enabled(
    state: tauri::State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<()> {
    {
        let mut disabled = state
            .disabled_devices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if enabled {
            disabled.remove(&id);
        } else {
            disabled.insert(id);
        }
    }
    persist_disabled(&state)?;
    Ok(())
}

/// How many segments the transcription pool has dropped this session.
#[tauri::command]
pub async fn capture_drop_count(state: tauri::State<'_, AppState>) -> Result<u64> {
    Ok(state
        .engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .map_or(0, super::super::audio::CaptureEngine::dropped_segments))
}

fn persist_disabled(state: &tauri::State<'_, AppState>) -> Result<()> {
    let joined = {
        let disabled = state
            .disabled_devices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ids: Vec<&str> = disabled.iter().map(String::as_str).collect();
        ids.sort_unstable();
        ids.join("\n")
    };
    let db = state
        .db
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    db.execute(
        "INSERT INTO settings(key, value) VALUES('capture.disabled_devices', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![joined],
    )?;
    Ok(())
}

/// Read the persisted disabled set at startup.
pub fn load_disabled(conn: &rusqlite::Connection) -> std::collections::HashSet<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'capture.disabled_devices'",
        [],
        |row| row.get::<_, String>(0),
    )
    .map(|v| {
        v.lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

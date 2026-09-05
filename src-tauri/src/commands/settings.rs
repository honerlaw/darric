use crate::{error::Result, state::AppState};

#[tauri::command]
pub async fn save_setting(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<()> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "INSERT INTO settings(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

#[tauri::command]
pub async fn get_setting(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(rusqlite::params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

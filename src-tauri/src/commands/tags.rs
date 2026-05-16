use crate::{error::Result, state::AppState};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

fn upsert_tag(db: &rusqlite::Connection, name: &str) -> rusqlite::Result<String> {
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT OR IGNORE INTO tags (id, name) VALUES (?1, ?2)",
        rusqlite::params![id, name],
    )?;
    db.query_row(
        "SELECT id FROM tags WHERE name = ?1",
        rusqlite::params![name],
        |r| r.get(0),
    )
}

pub fn session_tags(db: &rusqlite::Connection, session_id: &str) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = db.prepare(
        "SELECT t.id, t.name FROM tags t
         JOIN session_tags j ON j.tag_id = t.id
         WHERE j.session_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(rusqlite::params![session_id], |row| {
        Ok(Tag { id: row.get(0)?, name: row.get(1)? })
    })?;
    rows.collect()
}

pub fn note_tags(db: &rusqlite::Connection, note_id: &str) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = db.prepare(
        "SELECT t.id, t.name FROM tags t
         JOIN note_tags j ON j.tag_id = t.id
         WHERE j.note_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(rusqlite::params![note_id], |row| {
        Ok(Tag { id: row.get(0)?, name: row.get(1)? })
    })?;
    rows.collect()
}

pub fn task_tags(db: &rusqlite::Connection, task_id: &str) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = db.prepare(
        "SELECT t.id, t.name FROM tags t
         JOIN task_tags j ON j.tag_id = t.id
         WHERE j.task_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(rusqlite::params![task_id], |row| {
        Ok(Tag { id: row.get(0)?, name: row.get(1)? })
    })?;
    rows.collect()
}

#[tauri::command]
pub async fn list_tags(state: tauri::State<'_, AppState>) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare("SELECT id, name FROM tags ORDER BY name")?;
    let tags = stmt
        .query_map([], |row| Ok(Tag { id: row.get(0)?, name: row.get(1)? }))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tags)
}

#[tauri::command]
pub async fn add_tag_to_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
    tag_name: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    let tag_id = upsert_tag(&db, &tag_name)?;
    db.execute(
        "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
        rusqlite::params![session_id, tag_id],
    )?;
    Ok(session_tags(&db, &session_id)?)
}

#[tauri::command]
pub async fn remove_tag_from_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
    tag_id: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "DELETE FROM session_tags WHERE session_id = ?1 AND tag_id = ?2",
        rusqlite::params![session_id, tag_id],
    )?;
    Ok(session_tags(&db, &session_id)?)
}

#[tauri::command]
pub async fn add_tag_to_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    tag_name: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    let tag_id = upsert_tag(&db, &tag_name)?;
    db.execute(
        "INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?1, ?2)",
        rusqlite::params![note_id, tag_id],
    )?;
    Ok(note_tags(&db, &note_id)?)
}

#[tauri::command]
pub async fn remove_tag_from_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    tag_id: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "DELETE FROM note_tags WHERE note_id = ?1 AND tag_id = ?2",
        rusqlite::params![note_id, tag_id],
    )?;
    Ok(note_tags(&db, &note_id)?)
}

#[tauri::command]
pub async fn add_tag_to_task(
    state: tauri::State<'_, AppState>,
    task_id: String,
    tag_name: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    let tag_id = upsert_tag(&db, &tag_name)?;
    db.execute(
        "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
        rusqlite::params![task_id, tag_id],
    )?;
    Ok(task_tags(&db, &task_id)?)
}

#[tauri::command]
pub async fn remove_tag_from_task(
    state: tauri::State<'_, AppState>,
    task_id: String,
    tag_id: String,
) -> Result<Vec<Tag>> {
    let db = state.db.0.lock().unwrap();
    db.execute(
        "DELETE FROM task_tags WHERE task_id = ?1 AND tag_id = ?2",
        rusqlite::params![task_id, tag_id],
    )?;
    Ok(task_tags(&db, &task_id)?)
}

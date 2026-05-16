use crate::{commands::tags::Tag, error::Result, state::AppState};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<Tag>,
}

#[tauri::command]
pub async fn list_notes(state: tauri::State<'_, AppState>) -> Result<Vec<Note>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT n.id, n.title, n.body, n.created_at, n.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM note_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.note_id = n.id
           ), '[]') as tags_json
         FROM notes n ORDER BY n.updated_at DESC",
    )?;
    let notes = stmt
        .query_map([], |row| {
            let tags_json: String = row.get(5)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(notes)
}

#[tauri::command]
pub async fn get_note(state: tauri::State<'_, AppState>, id: String) -> Result<Note> {
    let db = state.db.0.lock().unwrap();
    let note = db.query_row(
        "SELECT n.id, n.title, n.body, n.created_at, n.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM note_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.note_id = n.id
           ), '[]') as tags_json
         FROM notes n WHERE n.id = ?1",
        rusqlite::params![id],
        |row| {
            let tags_json: String = row.get(5)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                tags,
            })
        },
    )?;
    Ok(note)
}

#[tauri::command]
pub async fn create_note(
    state: tauri::State<'_, AppState>,
    title: String,
    body: String,
) -> Result<Note> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    {
        let db = state.db.0.lock().unwrap();
        db.execute(
            "INSERT INTO notes(id, title, body, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![id, title, body, now],
        )?;
    }
    Ok(Note {
        id,
        title,
        body,
        created_at: now.clone(),
        updated_at: now,
        tags: vec![],
    })
}

#[tauri::command]
pub async fn update_note(
    state: tauri::State<'_, AppState>,
    id: String,
    title: String,
    body: String,
) -> Result<Note> {
    let now = Utc::now().to_rfc3339();
    let created_at: String;
    {
        let db = state.db.0.lock().unwrap();
        db.execute(
            "UPDATE notes SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![title, body, now, id],
        )?;
        created_at = db.query_row(
            "SELECT created_at FROM notes WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )?;
    }
    Ok(Note {
        id,
        title,
        body,
        created_at,
        updated_at: now,
        tags: vec![],
    })
}

#[tauri::command]
pub async fn delete_note(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.0.lock().unwrap();
    db.execute("DELETE FROM notes WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

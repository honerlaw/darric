use crate::{commands::tags::Tag, error::Result, state::AppState};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub col: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<Tag>,
}

#[tauri::command]
pub async fn list_tasks(state: tauri::State<'_, AppState>) -> Result<Vec<Task>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT tk.id, tk.title, tk.col, tk.position, tk.created_at, tk.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM task_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.task_id = tk.id
           ), '[]') as tags_json
         FROM tasks tk ORDER BY tk.col, tk.position ASC",
    )?;
    let tasks = stmt
        .query_map([], |row| {
            let tags_json: String = row.get(6)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                col: row.get(2)?,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tasks)
}

#[tauri::command]
pub async fn create_task(
    state: tauri::State<'_, AppState>,
    title: String,
    col: String,
) -> Result<Task> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let position: i64;
    {
        let db = state.db.0.lock().unwrap();
        position = db
            .query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM tasks WHERE col = ?1",
                rusqlite::params![col],
                |r| r.get(0),
            )
            .unwrap_or(0);
        db.execute(
            "INSERT INTO tasks(id, title, col, position, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?5)",
            rusqlite::params![id, title, col, position, now],
        )?;
    }
    Ok(Task {
        id,
        title,
        col,
        position,
        created_at: now.clone(),
        updated_at: now,
        tags: vec![],
    })
}

#[tauri::command]
pub async fn update_task(
    state: tauri::State<'_, AppState>,
    id: String,
    title: Option<String>,
    col: Option<String>,
    position: Option<i64>,
) -> Result<Task> {
    let now = Utc::now().to_rfc3339();
    let db = state.db.0.lock().unwrap();

    if let Some(ref t) = title {
        db.execute(
            "UPDATE tasks SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![t, now, id],
        )?;
    }
    if let Some(ref c) = col {
        db.execute(
            "UPDATE tasks SET col = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![c, now, id],
        )?;
    }
    if let Some(p) = position {
        db.execute(
            "UPDATE tasks SET position = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![p, now, id],
        )?;
    }

    let task = db.query_row(
        "SELECT tk.id, tk.title, tk.col, tk.position, tk.created_at, tk.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM task_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.task_id = tk.id
           ), '[]') as tags_json
         FROM tasks tk WHERE tk.id = ?1",
        rusqlite::params![id],
        |row| {
            let tags_json: String = row.get(6)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                col: row.get(2)?,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                tags,
            })
        },
    )?;
    Ok(task)
}

#[tauri::command]
pub async fn delete_task(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.0.lock().unwrap();
    db.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

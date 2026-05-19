use crate::{commands::tags::Tag, error::Result, state::AppState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultSession {
    pub id: String,
    pub topic: Option<String>,
    pub started_at: String,
    pub snippet: Option<String>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultNote {
    pub id: String,
    pub title: String,
    pub snippet: Option<String>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultTask {
    pub id: String,
    pub title: String,
    pub col: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub sessions: Vec<SearchResultSession>,
    pub notes: Vec<SearchResultNote>,
    pub tasks: Vec<SearchResultTask>,
}

#[tauri::command]
pub async fn search_all(state: tauri::State<'_, AppState>, query: String) -> Result<SearchResults> {
    if query.trim().is_empty() {
        return Ok(SearchResults {
            sessions: vec![],
            notes: vec![],
            tasks: vec![],
        });
    }
    let db = state.db.0.lock().unwrap();
    let pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = db.prepare(
        "SELECT DISTINCT s.id, s.topic, s.started_at,
           (SELECT tl.content FROM transcript_lines tl
            WHERE tl.session_id = s.id AND lower(tl.content) LIKE ?1 LIMIT 1) as snippet,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM session_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.session_id = s.id
           ), '[]') as tags_json
         FROM sessions s
         LEFT JOIN session_tags st ON st.session_id = s.id
         LEFT JOIN tags stag ON stag.id = st.tag_id
         WHERE lower(COALESCE(s.topic,'')) LIKE ?1
            OR lower(COALESCE(s.notes,'')) LIKE ?1
            OR lower(COALESCE(stag.name,'')) LIKE ?1
            OR EXISTS (SELECT 1 FROM transcript_lines tl
                       WHERE tl.session_id = s.id AND lower(tl.content) LIKE ?1)
         ORDER BY s.started_at DESC
         LIMIT 20",
    )?;
    let sessions = stmt
        .query_map(rusqlite::params![pattern], |row| {
            let tags_json: String = row.get(4)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(SearchResultSession {
                id: row.get(0)?,
                topic: row.get(1)?,
                started_at: row.get(2)?,
                snippet: row.get(3)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut stmt = db.prepare(
        "SELECT n.id, n.title,
           CASE WHEN lower(n.body) LIKE ?1 THEN substr(n.body, 1, 160) ELSE NULL END as snippet,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM note_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.note_id = n.id
           ), '[]') as tags_json
         FROM notes n
         LEFT JOIN note_tags nt ON nt.note_id = n.id
         LEFT JOIN tags ntag ON ntag.id = nt.tag_id
         WHERE lower(n.title) LIKE ?1
            OR lower(n.body) LIKE ?1
            OR lower(COALESCE(ntag.name,'')) LIKE ?1
         GROUP BY n.id
         ORDER BY n.updated_at DESC
         LIMIT 20",
    )?;
    let notes = stmt
        .query_map(rusqlite::params![pattern], |row| {
            let tags_json: String = row.get(3)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(SearchResultNote {
                id: row.get(0)?,
                title: row.get(1)?,
                snippet: row.get(2)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut stmt = db.prepare(
        "SELECT tk.id, tk.title, tk.col,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM task_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.task_id = tk.id
           ), '[]') as tags_json
         FROM tasks tk
         LEFT JOIN task_tags tt ON tt.task_id = tk.id
         LEFT JOIN tags ttag ON ttag.id = tt.tag_id
         WHERE lower(tk.title) LIKE ?1
            OR lower(COALESCE(ttag.name,'')) LIKE ?1
         GROUP BY tk.id
         ORDER BY tk.updated_at DESC
         LIMIT 20",
    )?;
    let tasks = stmt
        .query_map(rusqlite::params![pattern], |row| {
            let tags_json: String = row.get(3)?;
            let tags = serde_json::from_str::<Vec<Tag>>(&tags_json).unwrap_or_default();
            Ok(SearchResultTask {
                id: row.get(0)?,
                title: row.get(1)?,
                col: row.get(2)?,
                tags,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(SearchResults {
        sessions,
        notes,
        tasks,
    })
}

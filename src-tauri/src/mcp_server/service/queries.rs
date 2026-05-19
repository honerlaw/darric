use crate::state::DbConn;
use serde_json::{json, Value};

use super::{ByTagArgs, ListMeetingsArgs, ListNotesArgs, ListTasksArgs, TimelineArgs};

type DbResult<T> = Result<T, rusqlite::Error>;

fn tags_array_for(arr: &str) -> Value {
    serde_json::from_str(arr).unwrap_or_else(|_| json!([]))
}

pub fn list_notes(db: &DbConn, args: &ListNotesArgs) -> DbResult<Value> {
    let limit = args.limit.unwrap_or(50).min(500);
    let offset = args.offset.unwrap_or(0);
    let conn = db.0.lock().expect("db mutex poisoned");
    let mut stmt = conn.prepare(
        "SELECT n.id, n.title, n.body, n.created_at, n.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM note_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.note_id = n.id
           ), '[]') as tags_json
         FROM notes n
         ORDER BY n.updated_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit, offset], |row| {
            let tags_json: String = row.get(5)?;
            let body: String = row.get(2)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "preview": preview(&body, 240),
                "created_at": row.get::<_, String>(3)?,
                "updated_at": row.get::<_, String>(4)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;
    Ok(json!({ "notes": rows }))
}

pub fn get_note(db: &DbConn, id: &str) -> DbResult<Value> {
    let conn = db.0.lock().expect("db mutex poisoned");
    let note = conn.query_row(
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
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "body": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "updated_at": row.get::<_, String>(4)?,
                "tags": tags_array_for(&tags_json),
            }))
        },
    )?;
    Ok(note)
}

pub fn search(db: &DbConn, query: &str, limit: u32) -> DbResult<Value> {
    let conn = db.0.lock().expect("db mutex poisoned");
    let pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = conn.prepare(
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
         LIMIT ?2",
    )?;
    let meetings = stmt
        .query_map(rusqlite::params![pattern, limit], |row| {
            let tags_json: String = row.get(4)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "topic": row.get::<_, Option<String>>(1)?,
                "started_at": row.get::<_, String>(2)?,
                "snippet": row.get::<_, Option<String>>(3)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT n.id, n.title,
           CASE WHEN lower(n.body) LIKE ?1 THEN substr(n.body, 1, 240) ELSE NULL END as snippet,
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
         LIMIT ?2",
    )?;
    let notes = stmt
        .query_map(rusqlite::params![pattern, limit], |row| {
            let tags_json: String = row.get(3)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "snippet": row.get::<_, Option<String>>(2)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;

    let mut stmt = conn.prepare(
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
         LIMIT ?2",
    )?;
    let tasks = stmt
        .query_map(rusqlite::params![pattern, limit], |row| {
            let tags_json: String = row.get(3)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "col": row.get::<_, String>(2)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;

    Ok(json!({
        "query": query,
        "meetings": meetings,
        "notes": notes,
        "tasks": tasks,
    }))
}

pub fn list_meetings(db: &DbConn, args: ListMeetingsArgs) -> DbResult<Value> {
    let limit = args.limit.unwrap_or(50).min(500);
    let since = args.since.unwrap_or_default();
    let until = args.until.unwrap_or_default();
    let conn = db.0.lock().expect("db mutex poisoned");
    let mut stmt = conn.prepare(
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
         WHERE (?1 = '' OR s.started_at >= ?1)
           AND (?2 = '' OR s.started_at <= ?2)
         GROUP BY s.id
         ORDER BY s.started_at DESC
         LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![since, until, limit], |row| {
            let tags_json: String = row.get(7)?;
            let notes: Option<String> = row.get(5)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "topic": row.get::<_, Option<String>>(1)?,
                "started_at": row.get::<_, String>(2)?,
                "ended_at": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "notes_preview": notes.as_deref().map(|s| preview(s, 240)),
                "recorded_minutes": row.get::<_, i64>(6)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;
    Ok(json!({ "meetings": rows }))
}

pub fn get_meeting(db: &DbConn, id: &str, max_transcript_bytes: usize) -> DbResult<Value> {
    let conn = db.0.lock().expect("db mutex poisoned");
    let meta = conn.query_row(
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
        rusqlite::params![id],
        |row| {
            let tags_json: String = row.get(7)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "topic": row.get::<_, Option<String>>(1)?,
                "started_at": row.get::<_, String>(2)?,
                "ended_at": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "notes": row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                "recorded_minutes": row.get::<_, i64>(6)?,
                "tags": tags_array_for(&tags_json),
            }))
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT source, speaker_label, content, recorded_at
         FROM transcript_lines
         WHERE session_id = ?1
         ORDER BY recorded_at ASC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<DbResult<Vec<_>>>()?;

    let mut lines = Vec::with_capacity(rows.len());
    let mut bytes_so_far: usize = 0;
    let mut truncated_at: Option<usize> = None;
    for (source, speaker, content, recorded_at) in rows {
        let next = bytes_so_far.saturating_add(content.len()).saturating_add(1);
        if next > max_transcript_bytes {
            truncated_at = Some(bytes_so_far);
            break;
        }
        bytes_so_far = next;
        lines.push(json!({
            "source": source,
            "speaker": speaker,
            "content": content,
            "recorded_at": recorded_at,
        }));
    }

    let mut out = meta;
    out["transcript"] = json!(lines);
    if let Some(at) = truncated_at {
        out["truncated_at_bytes"] = json!(at);
    }
    Ok(out)
}

pub fn list_tasks(db: &DbConn, args: ListTasksArgs) -> DbResult<Value> {
    let status = args.status.unwrap_or_default();
    let tag = args.tag.unwrap_or_default();
    let conn = db.0.lock().expect("db mutex poisoned");
    let mut stmt = conn.prepare(
        "SELECT tk.id, tk.title, tk.col, tk.position, tk.created_at, tk.updated_at,
           COALESCE((
             SELECT json_group_array(json_object('id', t.id, 'name', t.name))
             FROM task_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.task_id = tk.id
           ), '[]') as tags_json
         FROM tasks tk
         WHERE (?1 = '' OR tk.col = ?1)
           AND (?2 = '' OR EXISTS (
             SELECT 1 FROM task_tags j JOIN tags t ON t.id = j.tag_id
             WHERE j.task_id = tk.id AND t.name = ?2
           ))
         ORDER BY tk.col, tk.position ASC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![status, tag], |row| {
            let tags_json: String = row.get(6)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "col": row.get::<_, String>(2)?,
                "position": row.get::<_, i64>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "updated_at": row.get::<_, String>(5)?,
                "tags": tags_array_for(&tags_json),
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;
    Ok(json!({ "tasks": rows }))
}

pub fn list_tags(db: &DbConn) -> DbResult<Value> {
    let conn = db.0.lock().expect("db mutex poisoned");
    let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
            }))
        })?
        .collect::<DbResult<Vec<_>>>()?;
    Ok(json!({ "tags": rows }))
}

pub fn by_tag(db: &DbConn, args: &ByTagArgs) -> DbResult<Value> {
    let include = |t: &str| -> bool {
        args.types
            .as_ref()
            .is_none_or(|xs| xs.iter().any(|x| x == t))
    };
    let conn = db.0.lock().expect("db mutex poisoned");

    let notes: Vec<Value> = if include("notes") {
        let mut stmt = conn.prepare(
            "SELECT n.id, n.title, n.updated_at
             FROM notes n
             JOIN note_tags j ON j.note_id = n.id
             JOIN tags t ON t.id = j.tag_id
             WHERE t.name = ?1
             ORDER BY n.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![args.tag], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "updated_at": row.get::<_, String>(2)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    let meetings: Vec<Value> = if include("meetings") {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.topic, s.started_at
             FROM sessions s
             JOIN session_tags j ON j.session_id = s.id
             JOIN tags t ON t.id = j.tag_id
             WHERE t.name = ?1
             ORDER BY s.started_at DESC",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![args.tag], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "topic": row.get::<_, Option<String>>(1)?,
                    "started_at": row.get::<_, String>(2)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    let tasks: Vec<Value> = if include("tasks") {
        let mut stmt = conn.prepare(
            "SELECT tk.id, tk.title, tk.col, tk.updated_at
             FROM tasks tk
             JOIN task_tags j ON j.task_id = tk.id
             JOIN tags t ON t.id = j.tag_id
             WHERE t.name = ?1
             ORDER BY tk.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![args.tag], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "col": row.get::<_, String>(2)?,
                    "updated_at": row.get::<_, String>(3)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    Ok(json!({
        "tag": args.tag,
        "notes": notes,
        "meetings": meetings,
        "tasks": tasks,
    }))
}

pub fn timeline(db: &DbConn, args: TimelineArgs) -> DbResult<Value> {
    let limit = args.limit.unwrap_or(100).min(1000);
    let from = args.from.unwrap_or_default();
    let to = args.to.unwrap_or_default();
    let include = |t: &str| -> bool {
        args.types
            .as_ref()
            .is_none_or(|xs| xs.iter().any(|x| x == t))
    };
    let conn = db.0.lock().expect("db mutex poisoned");

    let mut entries: Vec<Value> = Vec::new();

    if include("notes") {
        let mut stmt = conn.prepare(
            "SELECT id, title, updated_at FROM notes
             WHERE (?1 = '' OR updated_at >= ?1)
               AND (?2 = '' OR updated_at <= ?2)
             ORDER BY updated_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![from, to, limit], |row| {
                Ok(json!({
                    "kind": "note",
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "at": row.get::<_, String>(2)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        entries.extend(rows);
    }

    if include("meetings") {
        let mut stmt = conn.prepare(
            "SELECT id, topic, started_at FROM sessions
             WHERE (?1 = '' OR started_at >= ?1)
               AND (?2 = '' OR started_at <= ?2)
             ORDER BY started_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![from, to, limit], |row| {
                Ok(json!({
                    "kind": "meeting",
                    "id": row.get::<_, String>(0)?,
                    "topic": row.get::<_, Option<String>>(1)?,
                    "at": row.get::<_, String>(2)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        entries.extend(rows);
    }

    if include("tasks") {
        let mut stmt = conn.prepare(
            "SELECT id, title, col, updated_at FROM tasks
             WHERE (?1 = '' OR updated_at >= ?1)
               AND (?2 = '' OR updated_at <= ?2)
             ORDER BY updated_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![from, to, limit], |row| {
                Ok(json!({
                    "kind": "task",
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "col": row.get::<_, String>(2)?,
                    "at": row.get::<_, String>(3)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        entries.extend(rows);
    }

    if include("chat") {
        let mut stmt = conn.prepare(
            "SELECT id, role, content, created_at FROM chat_messages
             WHERE (?1 = '' OR created_at >= ?1)
               AND (?2 = '' OR created_at <= ?2)
             ORDER BY created_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![from, to, limit], |row| {
                let content: String = row.get(2)?;
                Ok(json!({
                    "kind": "chat",
                    "id": row.get::<_, String>(0)?,
                    "role": row.get::<_, String>(1)?,
                    "preview": preview(&content, 240),
                    "at": row.get::<_, String>(3)?,
                }))
            })?
            .collect::<DbResult<Vec<_>>>()?;
        entries.extend(rows);
    }

    entries.sort_by(|a, b| b["at"].as_str().cmp(&a["at"].as_str()));
    entries.truncate(limit as usize);
    Ok(json!({ "entries": entries }))
}

fn preview(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max_chars).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::state::DbConn;
    use rusqlite::{params, Connection};
    use rusqlite_migration::{Migrations, M};
    use std::sync::Mutex;

    fn test_db() -> DbConn {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        Migrations::new(vec![
            M::up(include_str!("../../../migrations/001_initial.sql")),
            M::up(include_str!("../../../migrations/002_notes_tasks.sql")),
            M::up(include_str!(
                "../../../migrations/003_reset_notes_tasks.sql"
            )),
            M::up(include_str!("../../../migrations/004_session_notes.sql")),
            M::up(include_str!(
                "../../../migrations/005_recording_segments.sql"
            )),
            M::up(include_str!("../../../migrations/006_chat.sql")),
            M::up(include_str!("../../../migrations/007_speaker_label.sql")),
            M::up(include_str!("../../../migrations/008_tags.sql")),
        ])
        .to_latest(&mut conn)
        .unwrap();
        DbConn(Mutex::new(conn))
    }

    fn seed_note(db: &DbConn, id: &str, title: &str, body: &str, ts: &str) {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO notes(id, title, body, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?4)",
            params![id, title, body, ts],
        )
        .unwrap();
    }

    fn seed_session(db: &DbConn, id: &str, topic: Option<&str>, started_at: &str) {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions(id, topic, started_at, created_at) VALUES(?1, ?2, ?3, ?3)",
            params![id, topic, started_at],
        )
        .unwrap();
    }

    fn seed_transcript(db: &DbConn, session_id: &str, content: &str, recorded_at: &str) {
        let conn = db.0.lock().unwrap();
        let line_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO transcript_lines(id, session_id, source, content, recorded_at)
             VALUES(?1, ?2, 'mic', ?3, ?4)",
            params![line_id, session_id, content, recorded_at],
        )
        .unwrap();
    }

    fn seed_task(db: &DbConn, id: &str, title: &str, col: &str, position: i64, ts: &str) {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks(id, title, col, position, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, title, col, position, ts],
        )
        .unwrap();
    }

    fn tag_entity(db: &DbConn, kind: &str, entity_id: &str, tag_name: &str) {
        let conn = db.0.lock().unwrap();
        let tag_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO tags(id, name) VALUES(?1, ?2)",
            params![tag_id, tag_name],
        )
        .unwrap();
        let tag_id: String = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1",
                params![tag_name],
                |r| r.get(0),
            )
            .unwrap();
        let table = match kind {
            "note" => "note_tags",
            "session" => "session_tags",
            "task" => "task_tags",
            _ => panic!("unknown kind"),
        };
        let col = match kind {
            "note" => "note_id",
            "session" => "session_id",
            "task" => "task_id",
            _ => unreachable!(),
        };
        conn.execute(
            &format!("INSERT INTO {table}({col}, tag_id) VALUES(?1, ?2)"),
            params![entity_id, tag_id],
        )
        .unwrap();
    }

    #[test]
    fn list_notes_returns_newest_first_with_tags() {
        let db = test_db();
        seed_note(&db, "n1", "Older", "Body 1", "2024-01-01T10:00:00Z");
        seed_note(&db, "n2", "Newer", "Body 2", "2024-01-02T10:00:00Z");
        tag_entity(&db, "note", "n2", "focus");

        let result = list_notes(
            &db,
            &ListNotesArgs {
                limit: None,
                offset: None,
            },
        )
        .unwrap();
        let notes = result["notes"].as_array().unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0]["id"], "n2");
        assert_eq!(notes[0]["preview"], "Body 2");
        assert_eq!(notes[0]["tags"][0]["name"], "focus");
        assert_eq!(notes[1]["id"], "n1");
    }

    #[test]
    fn list_notes_respects_limit_and_offset() {
        let db = test_db();
        for i in 0..5 {
            seed_note(
                &db,
                &format!("n{i}"),
                &format!("Title {i}"),
                "body",
                &format!("2024-01-0{}T10:00:00Z", i + 1),
            );
        }
        let result = list_notes(
            &db,
            &ListNotesArgs {
                limit: Some(2),
                offset: Some(1),
            },
        )
        .unwrap();
        let notes = result["notes"].as_array().unwrap();
        assert_eq!(notes.len(), 2);
        // newest first: n4, n3, n2, n1, n0; offset 1, limit 2 → n3, n2
        assert_eq!(notes[0]["id"], "n3");
        assert_eq!(notes[1]["id"], "n2");
    }

    #[test]
    fn get_note_returns_full_body() {
        let db = test_db();
        seed_note(&db, "n1", "Title", "Full body here", "2024-01-01T10:00:00Z");

        let result = get_note(&db, "n1").unwrap();
        assert_eq!(result["id"], "n1");
        assert_eq!(result["body"], "Full body here");
    }

    #[test]
    fn search_finds_across_notes_meetings_tasks() {
        let db = test_db();
        seed_note(
            &db,
            "n1",
            "Strategy doc",
            "Body about widgets",
            "2024-01-01T10:00:00Z",
        );
        seed_session(&db, "s1", Some("Widget brainstorm"), "2024-01-02T10:00:00Z");
        seed_transcript(
            &db,
            "s1",
            "we should ship the widget",
            "2024-01-02T10:01:00Z",
        );
        seed_task(&db, "t1", "Build widget", "todo", 0, "2024-01-03T10:00:00Z");

        let result = search(&db, "widget", 20).unwrap();
        assert!(!result["notes"].as_array().unwrap().is_empty());
        assert!(!result["meetings"].as_array().unwrap().is_empty());
        assert!(!result["tasks"].as_array().unwrap().is_empty());
        assert_eq!(result["query"], "widget");
    }

    #[test]
    fn search_is_case_insensitive() {
        let db = test_db();
        seed_note(
            &db,
            "n1",
            "Mixed Case",
            "Hello WORLD",
            "2024-01-01T10:00:00Z",
        );
        let result = search(&db, "world", 20).unwrap();
        assert_eq!(result["notes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn list_meetings_filters_by_since() {
        let db = test_db();
        seed_session(&db, "s1", Some("Old"), "2024-01-01T10:00:00Z");
        seed_session(&db, "s2", Some("New"), "2024-02-01T10:00:00Z");

        let result = list_meetings(
            &db,
            ListMeetingsArgs {
                limit: None,
                since: Some("2024-01-15T00:00:00Z".to_owned()),
                until: None,
            },
        )
        .unwrap();
        let meetings = result["meetings"].as_array().unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0]["id"], "s2");
    }

    #[test]
    fn get_meeting_includes_transcript() {
        let db = test_db();
        seed_session(&db, "s1", Some("Topic"), "2024-01-01T10:00:00Z");
        seed_transcript(&db, "s1", "first line", "2024-01-01T10:00:01Z");
        seed_transcript(&db, "s1", "second line", "2024-01-01T10:00:02Z");

        let result = get_meeting(&db, "s1", 1024).unwrap();
        let lines = result["transcript"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["content"], "first line");
        assert!(result.get("truncated_at_bytes").is_none());
    }

    #[test]
    fn get_meeting_truncates_when_over_cap() {
        let db = test_db();
        seed_session(&db, "s1", None, "2024-01-01T10:00:00Z");
        // 200-byte line, cap at 250 bytes — second line should not fit.
        let big = "x".repeat(200);
        seed_transcript(&db, "s1", &big, "2024-01-01T10:00:01Z");
        seed_transcript(&db, "s1", &big, "2024-01-01T10:00:02Z");

        let result = get_meeting(&db, "s1", 250).unwrap();
        let lines = result["transcript"].as_array().unwrap();
        assert_eq!(lines.len(), 1);
        assert!(result["truncated_at_bytes"].is_number());
    }

    #[test]
    fn list_tasks_filters_by_status_and_tag() {
        let db = test_db();
        seed_task(&db, "t1", "Todo task", "todo", 0, "2024-01-01T10:00:00Z");
        seed_task(&db, "t2", "Doing task", "doing", 0, "2024-01-01T10:00:00Z");
        seed_task(&db, "t3", "Tagged todo", "todo", 1, "2024-01-01T10:00:00Z");
        tag_entity(&db, "task", "t3", "urgent");

        let by_status = list_tasks(
            &db,
            ListTasksArgs {
                status: Some("todo".to_owned()),
                tag: None,
            },
        )
        .unwrap();
        assert_eq!(by_status["tasks"].as_array().unwrap().len(), 2);

        let by_tag = list_tasks(
            &db,
            ListTasksArgs {
                status: None,
                tag: Some("urgent".to_owned()),
            },
        )
        .unwrap();
        let tagged = by_tag["tasks"].as_array().unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0]["id"], "t3");
    }

    #[test]
    fn list_tags_returns_alphabetical() {
        let db = test_db();
        seed_note(&db, "n1", "T", "B", "2024-01-01T10:00:00Z");
        tag_entity(&db, "note", "n1", "zebra");
        tag_entity(&db, "note", "n1", "apple");

        let result = list_tags(&db).unwrap();
        let tags = result["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0]["name"], "apple");
        assert_eq!(tags[1]["name"], "zebra");
    }

    #[test]
    fn by_tag_returns_only_requested_types() {
        let db = test_db();
        seed_note(&db, "n1", "T", "B", "2024-01-01T10:00:00Z");
        seed_session(&db, "s1", Some("S"), "2024-01-01T10:00:00Z");
        seed_task(&db, "t1", "Task", "todo", 0, "2024-01-01T10:00:00Z");
        tag_entity(&db, "note", "n1", "focus");
        tag_entity(&db, "session", "s1", "focus");
        tag_entity(&db, "task", "t1", "focus");

        let only_notes = by_tag(
            &db,
            &ByTagArgs {
                tag: "focus".to_owned(),
                types: Some(vec!["notes".to_owned()]),
            },
        )
        .unwrap();
        assert_eq!(only_notes["notes"].as_array().unwrap().len(), 1);
        assert_eq!(only_notes["meetings"].as_array().unwrap().len(), 0);
        assert_eq!(only_notes["tasks"].as_array().unwrap().len(), 0);

        let all = by_tag(
            &db,
            &ByTagArgs {
                tag: "focus".to_owned(),
                types: None,
            },
        )
        .unwrap();
        assert_eq!(all["notes"].as_array().unwrap().len(), 1);
        assert_eq!(all["meetings"].as_array().unwrap().len(), 1);
        assert_eq!(all["tasks"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn timeline_merges_and_sorts_descending() {
        let db = test_db();
        seed_note(&db, "n1", "Note", "B", "2024-01-02T10:00:00Z");
        seed_session(&db, "s1", Some("Meeting"), "2024-01-01T10:00:00Z");
        seed_task(&db, "t1", "Task", "todo", 0, "2024-01-03T10:00:00Z");

        let result = timeline(
            &db,
            TimelineArgs {
                from: None,
                to: None,
                types: None,
                limit: None,
            },
        )
        .unwrap();
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["kind"], "task");
        assert_eq!(entries[1]["kind"], "note");
        assert_eq!(entries[2]["kind"], "meeting");
    }

    #[test]
    fn timeline_respects_types_filter() {
        let db = test_db();
        seed_note(&db, "n1", "Note", "B", "2024-01-01T10:00:00Z");
        seed_session(&db, "s1", None, "2024-01-01T10:00:00Z");
        seed_task(&db, "t1", "Task", "todo", 0, "2024-01-01T10:00:00Z");

        let result = timeline(
            &db,
            TimelineArgs {
                from: None,
                to: None,
                types: Some(vec!["notes".to_owned(), "tasks".to_owned()]),
                limit: None,
            },
        )
        .unwrap();
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        for e in entries {
            let kind = e["kind"].as_str().unwrap();
            assert!(kind == "note" || kind == "task");
        }
    }
}

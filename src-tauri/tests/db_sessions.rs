mod common;

use rusqlite::params;

const SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440010";
const SESSION_ID2: &str = "550e8400-e29b-41d4-a716-446655440011";
const TS_EARLY: &str = "2024-01-01T09:00:00Z";
const TS_LATE: &str = "2024-01-01T14:00:00Z";

fn insert_session(
    conn: &rusqlite::Connection,
    id: &str,
    topic: Option<&str>,
    started_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sessions(id, topic, started_at, created_at) VALUES(?1, ?2, ?3, ?3)",
        params![id, topic, started_at],
    )?;
    Ok(())
}

#[test]
fn insert_and_retrieve() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_session(&conn, SESSION_ID, Some("Q1 Planning"), TS_EARLY)?;

    let (id, topic, started_at): (String, Option<String>, String) = conn.query_row(
        "SELECT id, topic, started_at FROM sessions WHERE id = ?1",
        params![SESSION_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    assert_eq!(id, SESSION_ID);
    assert_eq!(topic.as_deref(), Some("Q1 Planning"));
    assert_eq!(started_at, TS_EARLY);
    Ok(())
}

#[test]
fn session_with_null_topic() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_session(&conn, SESSION_ID, None, TS_EARLY)?;

    let topic: Option<String> = conn.query_row(
        "SELECT topic FROM sessions WHERE id = ?1",
        params![SESSION_ID],
        |row| row.get(0),
    )?;
    assert!(topic.is_none());
    Ok(())
}

#[test]
fn ended_at_set_on_stop() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_session(&conn, SESSION_ID, None, TS_EARLY)?;

    conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        params![TS_LATE, SESSION_ID],
    )?;

    let ended_at: Option<String> = conn.query_row(
        "SELECT ended_at FROM sessions WHERE id = ?1",
        params![SESSION_ID],
        |row| row.get(0),
    )?;
    assert_eq!(ended_at.as_deref(), Some(TS_LATE));
    Ok(())
}

#[test]
fn list_ordered_by_started_at_desc() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_session(&conn, SESSION_ID, Some("Morning"), TS_EARLY)?;
    insert_session(&conn, SESSION_ID2, Some("Afternoon"), TS_LATE)?;

    let mut stmt = conn.prepare("SELECT id FROM sessions ORDER BY started_at DESC")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert_eq!(ids.first().map(String::as_str), Some(SESSION_ID2)); // later session first
    assert_eq!(ids.get(1).map(String::as_str), Some(SESSION_ID));
    Ok(())
}

#[test]
fn strip_migration_removes_retired_tables() -> rusqlite::Result<()> {
    let conn = common::open_test_db();

    for table in [
        "notes",
        "tasks",
        "tags",
        "session_tags",
        "note_tags",
        "task_tags",
        "chat_messages",
    ] {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;
        assert_eq!(count, 0, "table {table} should have been dropped");
    }
    Ok(())
}

#[test]
fn strip_migration_removes_session_notes_column() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    let has_notes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'notes'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(has_notes, 0);
    Ok(())
}

#[test]
fn strip_migration_keeps_recorder_tables() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    for table in [
        "sessions",
        "transcript_lines",
        "recording_segments",
        "settings",
    ] {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;
        assert_eq!(count, 1, "table {table} should still exist");
    }
    Ok(())
}

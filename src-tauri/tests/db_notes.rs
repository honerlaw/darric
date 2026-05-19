mod common;

use rusqlite::params;

const NOTE_ID: &str = "550e8400-e29b-41d4-a716-446655440001";
const NOTE_ID2: &str = "550e8400-e29b-41d4-a716-446655440002";
const TAG_ID: &str = "550e8400-e29b-41d4-a716-446655440003";
const TS_EARLY: &str = "2024-01-01T10:00:00Z";
const TS_LATE: &str = "2024-01-02T10:00:00Z";

fn insert_note(
    conn: &rusqlite::Connection,
    id: &str,
    title: &str,
    body: &str,
    ts: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO notes(id, title, body, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?4)",
        params![id, title, body, ts],
    )?;
    Ok(())
}

#[test]
fn create_and_retrieve() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Hello", "World", TS_EARLY)?;

    let (id, title, body, created_at, updated_at): (String, String, String, String, String) = conn
        .query_row(
            "SELECT id, title, body, created_at, updated_at FROM notes WHERE id = ?1",
            params![NOTE_ID],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

    assert_eq!(id, NOTE_ID);
    assert_eq!(title, "Hello");
    assert_eq!(body, "World");
    assert_eq!(created_at, TS_EARLY);
    assert_eq!(updated_at, TS_EARLY);
    Ok(())
}

#[test]
fn update_preserves_created_at() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Original title", "Original body", TS_EARLY)?;

    conn.execute(
        "UPDATE notes SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
        params!["Updated title", "Updated body", TS_LATE, NOTE_ID],
    )?;

    let (title, body, created_at, updated_at): (String, String, String, String) = conn.query_row(
        "SELECT title, body, created_at, updated_at FROM notes WHERE id = ?1",
        params![NOTE_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    assert_eq!(title, "Updated title");
    assert_eq!(body, "Updated body");
    assert_eq!(created_at, TS_EARLY); // must not change
    assert_eq!(updated_at, TS_LATE);
    Ok(())
}

#[test]
fn delete_removes_row() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Delete me", "", TS_EARLY)?;
    conn.execute("DELETE FROM notes WHERE id = ?1", params![NOTE_ID])?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0);
    Ok(())
}

#[test]
fn delete_cascades_to_note_tags() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Tagged note", "", TS_EARLY)?;
    conn.execute(
        "INSERT INTO tags(id, name) VALUES(?1, ?2)",
        params![TAG_ID, "work"],
    )?;
    conn.execute(
        "INSERT INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
        params![NOTE_ID, TAG_ID],
    )?;

    let linked: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(linked, 1);

    conn.execute("DELETE FROM notes WHERE id = ?1", params![NOTE_ID])?;

    let orphaned: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(orphaned, 0);
    Ok(())
}

#[test]
fn list_ordered_by_updated_at_desc() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    // Insert older note first
    insert_note(&conn, NOTE_ID, "Older", "", TS_EARLY)?;
    // Insert newer note second
    insert_note(&conn, NOTE_ID2, "Newer", "", TS_LATE)?;

    let mut stmt = conn.prepare("SELECT id FROM notes ORDER BY updated_at DESC")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert_eq!(ids.first().map(String::as_str), Some(NOTE_ID2));
    assert_eq!(ids.get(1).map(String::as_str), Some(NOTE_ID));
    Ok(())
}

#[test]
fn tag_add() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Tagged note", "", TS_EARLY)?;
    conn.execute(
        "INSERT INTO tags(id, name) VALUES(?1, ?2)",
        params![TAG_ID, "design"],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
        params![NOTE_ID, TAG_ID],
    )?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn tag_add_is_idempotent() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Tagged note", "", TS_EARLY)?;
    conn.execute(
        "INSERT INTO tags(id, name) VALUES(?1, ?2)",
        params![TAG_ID, "design"],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
        params![NOTE_ID, TAG_ID],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
        params![NOTE_ID, TAG_ID],
    )?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(
        count, 1,
        "duplicate INSERT OR IGNORE must not create a second row"
    );
    Ok(())
}

#[test]
fn tag_remove() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, NOTE_ID, "Tagged note", "", TS_EARLY)?;
    conn.execute(
        "INSERT INTO tags(id, name) VALUES(?1, ?2)",
        params![TAG_ID, "design"],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
        params![NOTE_ID, TAG_ID],
    )?;

    conn.execute(
        "DELETE FROM note_tags WHERE note_id = ?1 AND tag_id = ?2",
        params![NOTE_ID, TAG_ID],
    )?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
        params![NOTE_ID],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0);
    Ok(())
}

mod common;

use rusqlite::params;

const TS: &str = "2024-01-01T10:00:00Z";

fn insert_note(
    conn: &rusqlite::Connection,
    id: &str,
    title: &str,
    body: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO notes(id, title, body, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?4)",
        params![id, title, body, TS],
    )?;
    Ok(())
}

fn insert_task(
    conn: &rusqlite::Connection,
    id: &str,
    title: &str,
    col: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tasks(id, title, col, position, created_at, updated_at) VALUES(?1, ?2, ?3, 0, ?4, ?4)",
        params![id, title, col, TS],
    )?;
    Ok(())
}

fn note_match_count(conn: &rusqlite::Connection, pattern: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(DISTINCT n.id) FROM notes n
         LEFT JOIN note_tags nt ON nt.note_id = n.id
         LEFT JOIN tags ntag ON ntag.id = nt.tag_id
         WHERE lower(n.title) LIKE ?1
            OR lower(n.body) LIKE ?1
            OR lower(COALESCE(ntag.name,'')) LIKE ?1",
        params![pattern],
        |row| row.get(0),
    )
}

fn task_match_count(conn: &rusqlite::Connection, pattern: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(DISTINCT tk.id) FROM tasks tk
         LEFT JOIN task_tags tt ON tt.task_id = tk.id
         LEFT JOIN tags ttag ON ttag.id = tt.tag_id
         WHERE lower(tk.title) LIKE ?1
            OR lower(COALESCE(ttag.name,'')) LIKE ?1",
        params![pattern],
        |row| row.get(0),
    )
}

#[test]
fn note_title_match() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, "n1", "Weekly Standup", "notes here")?;

    let pattern = "%standup%";
    let count = note_match_count(&conn, pattern)?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn note_body_match() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, "n1", "Untitled", "Action items from retrospective")?;

    let pattern = "%retrospective%";
    let count = note_match_count(&conn, pattern)?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn note_search_is_case_insensitive() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, "n1", "DEPLOYMENT RUNBOOK", "Production steps")?;

    // Search with lowercase query against uppercase title
    let count = note_match_count(&conn, "%deployment%")?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn task_title_match() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "t1", "Write release notes", "doing")?;

    let pattern = "%release%";
    let count = task_match_count(&conn, pattern)?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn task_search_is_case_insensitive() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "t1", "Fix CRITICAL Bug", "todo")?;

    let count = task_match_count(&conn, "%critical%")?;
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn no_cross_contamination_between_entities() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, "n1", "Only in notes", "private content")?;
    insert_task(&conn, "t1", "Unrelated task", "todo")?;

    // "private" matches notes but not tasks
    let note_count = note_match_count(&conn, "%private%")?;
    let task_count = task_match_count(&conn, "%private%")?;
    assert_eq!(note_count, 1);
    assert_eq!(task_count, 0);
    Ok(())
}

#[test]
fn no_match_returns_zero() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_note(&conn, "n1", "Some note", "body text")?;
    insert_task(&conn, "t1", "Some task", "todo")?;

    let note_count = note_match_count(&conn, "%xyzzy_no_match%")?;
    let task_count = task_match_count(&conn, "%xyzzy_no_match%")?;
    assert_eq!(note_count, 0);
    assert_eq!(task_count, 0);
    Ok(())
}

#[test]
fn result_limit_enforced() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    // Insert 25 notes all matching the pattern — more than the 20-row limit
    for i in 0_u8..25 {
        let id = format!("note-{i:02}");
        insert_note(&conn, &id, &format!("Budget meeting {i}"), "")?;
    }

    // All 25 match without LIMIT
    let total = note_match_count(&conn, "%budget%")?;
    assert_eq!(total, 25);

    // The production search_all notes query caps results at 20 — use the exact shape
    let mut stmt = conn.prepare(
        "SELECT n.id FROM notes n
         LEFT JOIN note_tags nt ON nt.note_id = n.id
         LEFT JOIN tags ntag ON ntag.id = nt.tag_id
         WHERE lower(n.title) LIKE ?1
            OR lower(n.body) LIKE ?1
            OR lower(COALESCE(ntag.name,'')) LIKE ?1
         GROUP BY n.id
         ORDER BY n.updated_at DESC
         LIMIT 20",
    )?;
    let limited: Vec<String> = stmt
        .query_map(params!["%budget%"], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assert_eq!(limited.len(), 20);
    Ok(())
}

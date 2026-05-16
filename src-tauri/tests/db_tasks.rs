mod common;

use rusqlite::params;

const TS: &str = "2024-01-01T10:00:00Z";

fn next_position(conn: &rusqlite::Connection, col: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM tasks WHERE col = ?1",
        params![col],
        |row| row.get(0),
    )
}

fn insert_task(
    conn: &rusqlite::Connection,
    id: &str,
    title: &str,
    col: &str,
) -> rusqlite::Result<i64> {
    let position = next_position(conn, col)?;
    conn.execute(
        "INSERT INTO tasks(id, title, col, position, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, title, col, position, TS],
    )?;
    Ok(position)
}

#[test]
fn first_task_in_col_gets_position_zero() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    let pos = insert_task(&conn, "task-1", "First", "todo")?;
    assert_eq!(pos, 0);
    Ok(())
}

#[test]
fn second_task_in_same_col_gets_next_position() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "task-1", "First", "todo")?;
    let pos = insert_task(&conn, "task-2", "Second", "todo")?;
    assert_eq!(pos, 1);
    Ok(())
}

#[test]
fn task_in_different_col_starts_at_zero() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "task-1", "Todo task", "todo")?;
    insert_task(&conn, "task-2", "Another todo", "todo")?;
    let pos = insert_task(&conn, "task-3", "Doing task", "doing")?;
    assert_eq!(pos, 0);
    Ok(())
}

#[test]
fn update_title() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "task-1", "Original", "todo")?;

    conn.execute(
        "UPDATE tasks SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params!["Renamed", "2024-01-02T10:00:00Z", "task-1"],
    )?;

    let (title, col): (String, String) = conn.query_row(
        "SELECT title, col FROM tasks WHERE id = ?1",
        params!["task-1"],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(title, "Renamed");
    assert_eq!(col, "todo"); // col must not change
    Ok(())
}

#[test]
fn update_col_moves_task() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "task-1", "My task", "todo")?;

    conn.execute(
        "UPDATE tasks SET col = ?1, updated_at = ?2 WHERE id = ?3",
        params!["doing", "2024-01-02T10:00:00Z", "task-1"],
    )?;

    let col: String = conn.query_row(
        "SELECT col FROM tasks WHERE id = ?1",
        params!["task-1"],
        |row| row.get(0),
    )?;
    assert_eq!(col, "doing");
    Ok(())
}

#[test]
fn delete_removes_task() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    insert_task(&conn, "task-1", "Temporary", "todo")?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", params!["task-1"])?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE id = ?1",
        params!["task-1"],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0);
    Ok(())
}

#[test]
fn list_ordered_by_col_then_position() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    // Insert in scrambled order
    insert_task(&conn, "task-b", "Doing second", "doing")?;
    insert_task(&conn, "task-a", "Doing first", "doing")?;
    insert_task(&conn, "task-c", "Todo first", "todo")?;

    let mut stmt =
        conn.prepare("SELECT id FROM tasks ORDER BY col, position ASC")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // "doing" col comes before "todo" alphabetically; positions within each col are 0, 1
    assert_eq!(ids.first().map(String::as_str), Some("task-b")); // doing, pos 0
    assert_eq!(ids.get(1).map(String::as_str), Some("task-a")); // doing, pos 1
    assert_eq!(ids.get(2).map(String::as_str), Some("task-c")); // todo, pos 0
    Ok(())
}

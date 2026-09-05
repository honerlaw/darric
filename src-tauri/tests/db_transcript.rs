//! Migration 010 rebuilds `transcript_lines` to attribute each line to the
//! device that produced it. SQLite cannot drop the old `CHECK(source IN
//! ('mic','speaker'))` constraint, so the table is recreated and the rows
//! copied — which makes the backfill worth testing directly.

mod common;

use rusqlite::params;

const SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440020";

fn seed_session(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sessions(id, topic, started_at, created_at)
         VALUES(?1, 'Legacy', '2024-01-01T09:00:00Z', '2024-01-01T09:00:00Z')",
        params![SESSION_ID],
    )?;
    Ok(())
}

#[test]
fn schema_has_device_attribution_columns() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('transcript_lines')")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    for expected in ["device_id", "device_name", "direction"] {
        assert!(
            cols.iter().any(|c| c == expected),
            "missing {expected} in {cols:?}"
        );
    }
    Ok(())
}

#[test]
fn the_old_source_and_speaker_label_columns_are_gone() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('transcript_lines')")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert!(!cols.iter().any(|c| c == "source"), "source should be gone");
    assert!(
        !cols.iter().any(|c| c == "speaker_label"),
        "speaker_label went with the MFCC tracker"
    );
    Ok(())
}

#[test]
fn direction_is_constrained_to_input_or_output() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    seed_session(&conn)?;

    let bad = conn.execute(
        "INSERT INTO transcript_lines(id, session_id, device_id, device_name, direction, content, recorded_at)
         VALUES('x', ?1, 'd', 'D', 'sideways', 'hi', '2024-01-01T09:00:01Z')",
        params![SESSION_ID],
    );
    assert!(bad.is_err(), "an unknown direction must be rejected");

    for direction in ["input", "output"] {
        conn.execute(
            "INSERT INTO transcript_lines(id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES(?1, ?2, 'd', 'D', ?3, 'hi', '2024-01-01T09:00:01Z')",
            params![direction, SESSION_ID, direction],
        )?;
    }
    Ok(())
}

#[test]
fn rows_can_be_attributed_to_several_devices_in_one_session() -> rusqlite::Result<()> {
    // The whole point of the phase: two microphones recording concurrently.
    let conn = common::open_test_db();
    seed_session(&conn)?;

    for (id, dev) in [("l1", "Built-in Mic"), ("l2", "Rode NT-USB")] {
        conn.execute(
            "INSERT INTO transcript_lines(id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES(?1, ?2, ?3, ?3, 'input', 'hello', '2024-01-01T09:00:01Z')",
            params![id, SESSION_ID, dev],
        )?;
    }

    let devices: Vec<String> = conn
        .prepare("SELECT DISTINCT device_name FROM transcript_lines WHERE session_id = ?1 ORDER BY device_name")?
        .query_map(params![SESSION_ID], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assert_eq!(devices, vec!["Built-in Mic", "Rode NT-USB"]);
    Ok(())
}

#[test]
fn the_session_index_survives_the_table_rebuild() -> rusqlite::Result<()> {
    // Dropping and recreating a table drops its indexes with it; losing this one
    // would silently turn every transcript read into a full scan.
    let conn = common::open_test_db();
    let indexes: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='transcript_lines'",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert!(
        indexes.iter().any(|i| i == "idx_transcript_session"),
        "idx_transcript_session must be recreated, got {indexes:?}"
    );
    Ok(())
}

#[test]
fn the_foreign_key_to_sessions_survives_the_rebuild() -> rusqlite::Result<()> {
    let conn = common::open_test_db();
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    let orphan = conn.execute(
        "INSERT INTO transcript_lines(id, session_id, device_id, device_name, direction, content, recorded_at)
         VALUES('orphan', 'no-such-session', 'd', 'D', 'input', 'hi', '2024-01-01T09:00:01Z')",
        [],
    );
    assert!(orphan.is_err(), "a line must not outlive its session");
    Ok(())
}

/// Apply migrations 001..=009 only, so a database can be populated in its
/// pre-device-attribution shape and then migrated forward.
fn db_before_device_attribution() -> rusqlite::Connection {
    use rusqlite_migration::{Migrations, M};
    let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory DB");
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .expect("PRAGMA");
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_initial.sql")),
        M::up(include_str!("../migrations/002_notes_tasks.sql")),
        M::up(include_str!("../migrations/003_reset_notes_tasks.sql")),
        M::up(include_str!("../migrations/004_session_notes.sql")),
        M::up(include_str!("../migrations/005_recording_segments.sql")),
        M::up(include_str!("../migrations/006_chat.sql")),
        M::up(include_str!("../migrations/007_speaker_label.sql")),
        M::up(include_str!("../migrations/008_tags.sql")),
        M::up(include_str!("../migrations/009_strip_to_recorder.sql")),
    ])
    .to_latest(&mut conn)
    .expect("migrations 001..=009");
    conn
}

fn apply_device_attribution(conn: &rusqlite::Connection) {
    conn.execute_batch(include_str!("../migrations/010_device_attribution.sql"))
        .expect("migration 010");
}

#[test]
fn backfill_preserves_existing_transcript_rows() -> rusqlite::Result<()> {
    let conn = db_before_device_attribution();
    seed_session(&conn)?;
    conn.execute(
        "INSERT INTO transcript_lines(id, session_id, source, content, recorded_at, speaker_label)
         VALUES('old-1', ?1, 'mic', 'something I said', '2024-01-01T09:00:01Z', 'Speaker 1')",
        params![SESSION_ID],
    )?;

    apply_device_attribution(&conn);

    let (content, device, direction): (String, String, String) = conn.query_row(
        "SELECT content, device_name, direction FROM transcript_lines WHERE id = 'old-1'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(
        content, "something I said",
        "the transcript text must survive"
    );
    assert_eq!(direction, "input");
    assert!(
        device.contains("pre-upgrade"),
        "legacy rows are labelled as such, got {device}"
    );
    Ok(())
}

#[test]
fn backfill_maps_speaker_rows_to_the_output_direction() -> rusqlite::Result<()> {
    let conn = db_before_device_attribution();
    seed_session(&conn)?;
    conn.execute(
        "INSERT INTO transcript_lines(id, session_id, source, content, recorded_at)
         VALUES('old-2', ?1, 'speaker', 'what they said', '2024-01-01T09:00:02Z')",
        params![SESSION_ID],
    )?;

    apply_device_attribution(&conn);

    let direction: String = conn.query_row(
        "SELECT direction FROM transcript_lines WHERE id = 'old-2'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(direction, "output");
    Ok(())
}

#[test]
fn backfill_loses_no_rows() -> rusqlite::Result<()> {
    let conn = db_before_device_attribution();
    seed_session(&conn)?;
    for i in 0..25 {
        conn.execute(
            "INSERT INTO transcript_lines(id, session_id, source, content, recorded_at)
             VALUES(?1, ?2, ?3, 'line', '2024-01-01T09:00:00Z')",
            params![
                format!("row-{i}"),
                SESSION_ID,
                if i % 2 == 0 { "mic" } else { "speaker" }
            ],
        )?;
    }

    apply_device_attribution(&conn);

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM transcript_lines", [], |row| {
        row.get(0)
    })?;
    assert_eq!(count, 25, "every legacy row must be carried across");
    Ok(())
}

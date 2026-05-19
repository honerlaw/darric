use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

/// Returns a fresh in-memory SQLite connection with all migrations applied.
/// Each call is fully isolated — no shared state between tests.
///
/// When a new migration is added to `db/migrations.rs`, add the corresponding
/// `include_str!` entry here to keep the test schema in sync.
pub fn open_test_db() -> Connection {
    let mut conn = Connection::open_in_memory().expect("in-memory DB");
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .expect("PRAGMA");
    Migrations::new(vec![
        M::up(include_str!("../../migrations/001_initial.sql")),
        M::up(include_str!("../../migrations/002_notes_tasks.sql")),
        M::up(include_str!("../../migrations/003_reset_notes_tasks.sql")),
        M::up(include_str!("../../migrations/004_session_notes.sql")),
        M::up(include_str!("../../migrations/005_recording_segments.sql")),
        M::up(include_str!("../../migrations/006_chat.sql")),
        M::up(include_str!("../../migrations/007_speaker_label.sql")),
        M::up(include_str!("../../migrations/008_tags.sql")),
    ])
    .to_latest(&mut conn)
    .expect("migrations failed");
    conn
}

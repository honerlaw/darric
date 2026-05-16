use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
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
}

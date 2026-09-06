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
        M::up(include_str!("../../migrations/009_strip_to_recorder.sql")),
        M::up(include_str!("../../migrations/010_device_attribution.sql")),
        M::up(include_str!("../../migrations/011_drop_ai_settings.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migration_011_drops_the_stripped_ai_feature_settings() {
        let mut conn = Connection::open_in_memory().expect("in-memory DB");
        migrations()
            .to_version(&mut conn, 10)
            .expect("schema at 010");
        for (key, value) in [
            ("ai.provider", "claude"),
            ("ai.claude.api_key", "sk-not-a-real-key"),
            ("capture.disabled_devices", "[]"),
        ] {
            conn.execute(
                "INSERT INTO settings(key, value) VALUES(?1, ?2)",
                rusqlite::params![key, value],
            )
            .expect("seed");
        }

        migrations()
            .to_latest(&mut conn)
            .expect("migrate to latest");

        let ai_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key LIKE 'ai.%'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(ai_rows, 0, "no ai.* key survives");

        let kept: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'capture.disabled_devices'",
                [],
                |row| row.get(0),
            )
            .expect("unrelated settings are untouched");
        assert_eq!(kept, "[]");
    }
}

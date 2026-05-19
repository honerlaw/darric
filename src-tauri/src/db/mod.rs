mod migrations;

use crate::ai::{ChatMessage, ContentBlock, Role};

/// Read a single setting from the DB synchronously (used during startup before AppState exists).
pub fn get_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .ok()
}

/// Load persisted chat messages and convert to in-memory ChatMessage history.
pub fn load_chat_history(conn: &rusqlite::Connection) -> Vec<ChatMessage> {
    let Ok(mut stmt) =
        conn.prepare("SELECT role, content FROM chat_messages ORDER BY created_at ASC")
    else {
        return Vec::new();
    };

    stmt.query_map([], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        Ok((role, content))
    })
    .ok()
    .map(|rows| {
        rows.filter_map(Result::ok)
            .map(|(role, content)| {
                let r = if role == "assistant" {
                    Role::Assistant
                } else {
                    Role::User
                };
                ChatMessage {
                    role: r,
                    blocks: vec![ContentBlock::Text(content)],
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

pub fn open() -> crate::error::Result<rusqlite::Connection> {
    let home = std::env::var("HOME").map_or_else(
        |_| std::path::PathBuf::from("/tmp"),
        std::path::PathBuf::from,
    );
    let data_dir = home.join("Library/Application Support/darric");
    std::fs::create_dir_all(&data_dir)?;
    let mut conn = rusqlite::Connection::open(data_dir.join("darric.db"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrations::migrations()
        .to_latest(&mut conn)
        .map_err(|e| crate::error::AppError::Migration(e.to_string()))?;
    Ok(conn)
}

mod migrations;
pub mod sessions;

use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Where darric keeps its database.
pub fn path() -> PathBuf {
    let home = std::env::var("HOME").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
    home.join("Library/Application Support/darric")
        .join("darric.db")
}

/// Open (creating if needed) and migrate the app's database.
pub fn open() -> crate::error::Result<Connection> {
    open_at(&path())
}

/// Open (creating if needed) and migrate a database at an explicit path.
///
/// Split from [`open`] so a test can stand up a real on-disk database that a
/// second, read-only connection then observes — the arrangement the MCP server
/// runs under.
pub fn open_at(path: &Path) -> crate::error::Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrations::migrations()
        .to_latest(&mut conn)
        .map_err(|e| crate::error::AppError::Migration(e.to_string()))?;
    Ok(conn)
}

/// A second connection to a database the app has already opened and migrated.
///
/// Read-only at the SQLite level, so a writer cannot slip in by convention
/// drifting; and a separate handle, so the recorder's inserts never queue
/// behind an agent's query on the app connection's mutex. Runs no migrations:
/// the app connection did that before this is called, and a reader must not
/// rebuild tables underneath a writer.
pub fn open_read_only(path: &Path) -> crate::error::Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )?;
    Ok(conn)
}

/// An in-memory database at the current schema, for inline tests.
///
/// One helper reachable from every `#[cfg(test)]` module, built from the same
/// migration list production uses, so a new migration cannot leave a test
/// schema behind — the duplication `2026-05-19-decision-inline-tests-for-mcp-queries`
/// recorded as the cost of inline tests.
#[cfg(test)]
pub fn test_db() -> Connection {
    let mut conn = Connection::open_in_memory().expect("in-memory DB");
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .expect("PRAGMA");
    migrations::migrations()
        .to_latest(&mut conn)
        .expect("migrations");
    conn
}

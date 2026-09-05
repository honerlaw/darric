mod migrations;

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

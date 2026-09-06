//! Queries over `sessions` and `transcript_lines`.
//!
//! One home for this SQL, used by both the Tauri commands and the MCP tools,
//! so the UI's view of a recording and an agent's view of it cannot drift.

use rusqlite::{Connection, OptionalExtension, Row};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub topic: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub recorded_minutes: i64,
}

impl Session {
    /// A session with no `ended_at` is still being recorded — or, for the few
    /// seconds after Stop, still being flushed.
    pub const fn in_progress(&self) -> bool {
        self.ended_at.is_none()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptLine {
    /// The row's SQLite rowid: assigned under the insert lock, monotonic, and
    /// unique, which is what makes it usable as an exact paging cursor. It is
    /// only stable for the life of the app process — see [`transcript_page`].
    pub seq: i64,
    pub id: String,
    pub session_id: String,
    pub device_id: String,
    pub device_name: String,
    pub direction: String,
    pub content: String,
    pub recorded_at: String,
}

/// One page of a transcript in rowid order.
#[derive(Debug)]
pub struct TranscriptPage {
    pub lines: Vec<TranscriptLine>,
    /// Pass back as `after` to fetch what landed since. When nothing new has
    /// landed this echoes the caller's own cursor, so a poll loop can always
    /// feed it straight back; it is `None` only on an empty transcript read
    /// from the start.
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}

/// A transcript line that matched a search, with its session's identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    pub seq: i64,
    pub session_id: String,
    pub topic: Option<String>,
    pub started_at: String,
    pub device_name: String,
    pub direction: String,
    pub content: String,
    pub recorded_at: String,
}

/// Search results: sessions whose topic matched, and lines whose content did.
#[derive(Debug)]
pub struct SearchResults {
    pub sessions: Vec<Session>,
    pub lines: Vec<SearchHit>,
}

const SESSION_SELECT: &str = "SELECT s.id, s.topic, s.started_at, s.ended_at, s.created_at,
   COALESCE(CAST(SUM(
     (julianday(COALESCE(seg.ended_at, datetime('now'))) - julianday(seg.started_at)) * 1440
   ) AS INTEGER), 0) as recorded_minutes
 FROM sessions s
 LEFT JOIN recording_segments seg ON seg.session_id = s.id";

const LINE_SELECT: &str =
    "SELECT rowid, id, session_id, device_id, device_name, direction, content, recorded_at
 FROM transcript_lines";

fn map_session(row: &Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        topic: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        created_at: row.get(4)?,
        recorded_minutes: row.get(5)?,
    })
}

fn map_line(row: &Row<'_>) -> rusqlite::Result<TranscriptLine> {
    let direction: String = row.get(5)?;
    if crate::transcription::pool::Direction::parse(&direction).is_none() {
        log::warn!(
            "[session] transcript line {} has unknown direction {direction:?}",
            row.get::<_, String>(1)?
        );
    }
    Ok(TranscriptLine {
        seq: row.get(0)?,
        id: row.get(1)?,
        session_id: row.get(2)?,
        device_id: row.get(3)?,
        device_name: row.get(4)?,
        direction,
        content: row.get(6)?,
        recorded_at: row.get(7)?,
    })
}

/// Sessions newest-first. `limit: None` returns them all.
pub fn list_sessions(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> rusqlite::Result<Vec<Session>> {
    let sql =
        format!("{SESSION_SELECT} GROUP BY s.id ORDER BY s.started_at DESC LIMIT ?1 OFFSET ?2");
    let mut stmt = conn.prepare(&sql)?;
    // SQLite reads a negative LIMIT as "no limit".
    let limit = limit.map_or(-1, i64::from);
    let sessions = stmt
        .query_map(rusqlite::params![limit, offset], map_session)?
        .collect();
    sessions
}

pub fn get_session(conn: &Connection, id: &str) -> rusqlite::Result<Option<Session>> {
    let sql = format!("{SESSION_SELECT} WHERE s.id = ?1 GROUP BY s.id");
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_row(rusqlite::params![id], map_session)
        .optional()
}

/// Every line of a session in capture order, for display.
pub fn transcript_lines(
    conn: &Connection,
    session_id: &str,
) -> rusqlite::Result<Vec<TranscriptLine>> {
    let sql = format!("{LINE_SELECT} WHERE session_id = ?1 ORDER BY recorded_at ASC");
    let mut stmt = conn.prepare(&sql)?;
    let lines = stmt
        .query_map(rusqlite::params![session_id], map_line)?
        .collect();
    lines
}

/// Up to `limit` lines with `seq > after`, in rowid order.
///
/// Rowid order is insertion order — which is transcription-completion order,
/// not speech order, when two devices' whisper workers finish at different
/// times. It is chosen anyway because it is exact: `recorded_at` is stamped
/// before the insert lock is taken, so two lines can be timestamped in one
/// order and inserted in the other, and a timestamp cursor would skip one.
///
/// `transcript_lines` has a `TEXT` primary key, so its rowid is implicit and is
/// reassigned when the table is rebuilt: by `VACUUM`, which nothing here runs,
/// or by a create-copy-drop-rename migration such as 010. Both can only happen
/// inside [`super::open`] at startup, before any cursor has been handed out, so
/// a cursor is valid for the life of the app process and must not be persisted
/// across restarts.
pub fn transcript_page(
    conn: &Connection,
    session_id: &str,
    after: Option<i64>,
    limit: u32,
) -> rusqlite::Result<TranscriptPage> {
    let sql =
        format!("{LINE_SELECT} WHERE session_id = ?1 AND rowid > ?2 ORDER BY rowid ASC LIMIT ?3");
    let mut stmt = conn.prepare(&sql)?;
    // Fetch one past the page to learn whether there is more without a count.
    let fetch = i64::from(limit) + 1;
    let mut lines: Vec<TranscriptLine> = stmt
        .query_map(
            rusqlite::params![session_id, after.unwrap_or(0), fetch],
            map_line,
        )?
        .collect::<rusqlite::Result<_>>()?;
    let has_more = lines.len() > limit as usize;
    lines.truncate(limit as usize);
    let next_cursor = lines.last().map(|l| l.seq).or(after);
    Ok(TranscriptPage {
        lines,
        next_cursor,
        has_more,
    })
}

/// Substring search over line content and session topic, case-insensitive
/// for ASCII letters.
///
/// Both sides are folded the same way: SQLite's `lower()` is ASCII-only, so the
/// query is folded with `to_ascii_lowercase` rather than Unicode lowering —
/// otherwise "über" would miss "ÜBER" while matching "über", which is worse
/// than being consistently ASCII-only.
///
/// Sessions whose topic matches come back as sessions; lines whose content
/// matches come back as lines, newest first. Device names are never matched:
/// they appear on every line, so "MacBook" would return the whole corpus.
pub fn search(
    conn: &Connection,
    query: &str,
    session_id: Option<&str>,
    limit: u32,
) -> rusqlite::Result<SearchResults> {
    let pattern = like_pattern(query);

    let sql = format!(
        r"{SESSION_SELECT}
          WHERE lower(COALESCE(s.topic, '')) LIKE ?1 ESCAPE '\'
            AND (?2 IS NULL OR s.id = ?2)
          GROUP BY s.id ORDER BY s.started_at DESC LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let sessions = stmt
        .query_map(rusqlite::params![pattern, session_id, limit], map_session)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        r"SELECT l.rowid, l.session_id, s.topic, s.started_at,
                 l.device_name, l.direction, l.content, l.recorded_at
          FROM transcript_lines l
          JOIN sessions s ON s.id = l.session_id
          WHERE lower(l.content) LIKE ?1 ESCAPE '\'
            AND (?2 IS NULL OR l.session_id = ?2)
          ORDER BY l.rowid DESC LIMIT ?3",
    )?;
    let lines = stmt
        .query_map(rusqlite::params![pattern, session_id, limit], |row| {
            Ok(SearchHit {
                seq: row.get(0)?,
                session_id: row.get(1)?,
                topic: row.get(2)?,
                started_at: row.get(3)?,
                device_name: row.get(4)?,
                direction: row.get(5)?,
                content: row.get(6)?,
                recorded_at: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(SearchResults { sessions, lines })
}

/// `%query%`, ASCII-lower-cased to match SQLite's `lower()`, with LIKE's own
/// metacharacters escaped so a literal underscore or percent sign in a phrase
/// matches itself rather than anything.
fn like_pattern(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.to_ascii_lowercase().chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const TS: &str = "2024-01-01T09:00:00Z";

    fn seed_session(conn: &Connection, id: &str, topic: Option<&str>, ended_at: Option<&str>) {
        conn.execute(
            "INSERT INTO sessions(id, topic, started_at, created_at, ended_at)
             VALUES(?1, ?2, ?3, ?3, ?4)",
            rusqlite::params![id, topic, TS, ended_at],
        )
        .unwrap();
    }

    fn seed_line(conn: &Connection, id: &str, session: &str, content: &str, recorded_at: &str) {
        conn.execute(
            "INSERT INTO transcript_lines(
                 id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES(?1, ?2, 'dev', 'Dev', 'input', ?3, ?4)",
            rusqlite::params![id, session, content, recorded_at],
        )
        .unwrap();
    }

    #[test]
    fn transcript_page_walks_in_insertion_order_with_an_exact_cursor() {
        // Timestamps deliberately run backwards so the test fails if paging
        // ever switches to recorded_at: rowid is the contract.
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "first", "2024-01-01T09:00:03Z");
        seed_line(&conn, "l2", "s", "second", "2024-01-01T09:00:02Z");
        seed_line(&conn, "l3", "s", "third", "2024-01-01T09:00:01Z");

        let page = transcript_page(&conn, "s", None, 2).unwrap();
        let contents: Vec<_> = page.lines.iter().map(|l| l.content.as_str()).collect();
        assert_eq!(contents, ["first", "second"]);
        assert!(page.has_more);
        let cursor = page.next_cursor.unwrap();
        assert_eq!(cursor, page.lines[1].seq);

        let page = transcript_page(&conn, "s", Some(cursor), 2).unwrap();
        let contents: Vec<_> = page.lines.iter().map(|l| l.content.as_str()).collect();
        assert_eq!(contents, ["third"]);
        assert!(!page.has_more);

        let cursor = page.next_cursor;
        let page = transcript_page(&conn, "s", cursor, 2).unwrap();
        assert!(page.lines.is_empty());
        assert_eq!(
            page.next_cursor, cursor,
            "an empty page echoes the caller's cursor so a poll loop can pass it straight back"
        );
        assert!(!page.has_more);

        let page = transcript_page(&conn, "s", None, 2).unwrap();
        assert_eq!(
            page.lines.len(),
            2,
            "no cursor still starts from the beginning"
        );
    }

    #[test]
    fn transcript_page_is_scoped_to_its_session() {
        let conn = crate::db::test_db();
        seed_session(&conn, "a", None, None);
        seed_session(&conn, "b", None, None);
        seed_line(&conn, "l1", "a", "mine", TS);
        seed_line(&conn, "l2", "b", "theirs", TS);

        let page = transcript_page(&conn, "a", None, 10).unwrap();
        assert_eq!(page.lines.len(), 1);
        assert_eq!(page.lines[0].content, "mine");
    }

    #[test]
    fn transcript_lines_orders_by_capture_time_for_display() {
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "later", "2024-01-01T09:00:02Z");
        seed_line(&conn, "l2", "s", "earlier", "2024-01-01T09:00:01Z");

        let lines = transcript_lines(&conn, "s").unwrap();
        let contents: Vec<_> = lines.iter().map(|l| l.content.as_str()).collect();
        assert_eq!(contents, ["earlier", "later"]);
        assert!(
            lines.iter().all(|l| l.seq > 0),
            "every line carries its rowid"
        );
    }

    #[test]
    fn search_escapes_like_metacharacters() {
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "we are 100% done", TS);
        seed_line(&conn, "l2", "s", "we are 100 percent done", TS);
        seed_line(&conn, "l3", "s", "call snake_case", TS);
        seed_line(&conn, "l4", "s", "call snakeXcase", TS);

        let hits = search(&conn, "100%", None, 10).unwrap().lines;
        let contents: Vec<_> = hits.iter().map(|h| h.content.as_str()).collect();
        assert_eq!(contents, ["we are 100% done"]);

        let hits = search(&conn, "snake_case", None, 10).unwrap().lines;
        let contents: Vec<_> = hits.iter().map(|h| h.content.as_str()).collect();
        assert_eq!(contents, ["call snake_case"]);
    }

    #[test]
    fn search_is_case_insensitive_and_newest_first() {
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "Budget review", TS);
        seed_line(&conn, "l2", "s", "the BUDGET again", TS);

        let hits = search(&conn, "budget", None, 10).unwrap().lines;
        let contents: Vec<_> = hits.iter().map(|h| h.content.as_str()).collect();
        assert_eq!(contents, ["the BUDGET again", "Budget review"]);
    }

    #[test]
    fn search_folds_case_the_same_way_on_both_sides() {
        // SQLite's lower() is ASCII-only. Folding the query with Unicode
        // lowering would make "über" miss "ÜBER" yet hit "über"; folding both
        // sides ASCII-only is at least consistent: an exact-case non-ASCII
        // query always matches.
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "ÜBER alles", TS);

        let hits = search(&conn, "ÜBER", None, 10).unwrap().lines;
        assert_eq!(hits.len(), 1);
        let hits = search(&conn, "alles", None, 10).unwrap().lines;
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_matches_topics_as_sessions_not_lines() {
        let conn = crate::db::test_db();
        seed_session(&conn, "s", Some("Quarterly planning"), Some(TS));
        seed_line(&conn, "l1", "s", "unrelated words", TS);

        let results = search(&conn, "quarterly", None, 10).unwrap();
        assert_eq!(results.sessions.len(), 1);
        assert_eq!(results.sessions[0].id, "s");
        assert!(
            results.lines.is_empty(),
            "a topic match does not return every line"
        );
    }

    #[test]
    fn search_can_be_limited_to_one_session() {
        let conn = crate::db::test_db();
        seed_session(&conn, "a", Some("shared word"), None);
        seed_session(&conn, "b", Some("shared word"), None);
        seed_line(&conn, "l1", "a", "shared word here", TS);
        seed_line(&conn, "l2", "b", "shared word there", TS);

        let results = search(&conn, "shared", Some("b"), 10).unwrap();
        assert_eq!(results.sessions.len(), 1);
        assert_eq!(results.sessions[0].id, "b");
        assert_eq!(results.lines.len(), 1);
        assert_eq!(results.lines[0].session_id, "b");
    }

    #[test]
    fn search_never_matches_device_names() {
        let conn = crate::db::test_db();
        seed_session(&conn, "s", None, None);
        seed_line(&conn, "l1", "s", "nothing relevant", TS);

        let results = search(&conn, "dev", None, 10).unwrap();
        assert!(results.lines.is_empty());
        assert!(results.sessions.is_empty());
    }

    #[test]
    fn list_sessions_pages_newest_first() {
        let conn = crate::db::test_db();
        for (id, started) in [
            ("old", "2024-01-01T09:00:00Z"),
            ("new", "2024-01-02T09:00:00Z"),
        ] {
            conn.execute(
                "INSERT INTO sessions(id, topic, started_at, created_at) VALUES(?1, NULL, ?2, ?2)",
                rusqlite::params![id, started],
            )
            .unwrap();
        }

        let all = list_sessions(&conn, None, 0).unwrap();
        let ids: Vec<_> = all.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["new", "old"]);

        let second = list_sessions(&conn, Some(1), 1).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, "old");
    }

    #[test]
    fn get_session_reports_progress_from_ended_at() {
        let conn = crate::db::test_db();
        seed_session(&conn, "live", None, None);
        seed_session(&conn, "done", None, Some(TS));

        assert!(get_session(&conn, "live").unwrap().unwrap().in_progress());
        assert!(!get_session(&conn, "done").unwrap().unwrap().in_progress());
        assert!(get_session(&conn, "missing").unwrap().is_none());
    }
}

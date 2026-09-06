//! The MCP tool surface.

use crate::db::sessions::{self, Session};
use crate::state::DbConn;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use rusqlite::Connection;
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

/// A snapshot of the recorder, as the in-process server can see it.
///
/// A trait rather than a direct `AppState` read so the service can be built in
/// a test, where a Tauri `AppHandle` cannot be constructed.
pub trait LiveStatus: Send + Sync {
    fn snapshot(&self) -> LiveSnapshot;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveDevice {
    pub id: String,
    pub name: String,
    pub direction: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct LiveSnapshot {
    /// The session an engine is installed for, if one is.
    pub session_id: Option<String>,
    pub devices: Vec<LiveDevice>,
    pub dropped_segments: u64,
}

#[derive(Clone)]
pub struct DarricService {
    db: Arc<DbConn>,
    live: Arc<dyn LiveStatus>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSessionsArgs {
    /// Maximum number of sessions to return (default 50, max 500).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Number of sessions to skip, newest first (default 0).
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTranscriptArgs {
    /// Session id, from `list_sessions` or `status`.
    pub session_id: String,
    /// Return only lines with `seq` greater than this — pass the previous
    /// call's `next_cursor` to fetch what has landed since. Omit for the start.
    #[serde(default)]
    pub after: Option<i64>,
    /// Maximum number of lines to return (default 500, max 2000).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Substring to match, case-insensitively. Required, non-empty.
    pub query: String,
    /// Maximum number of matching lines and sessions to return (default 50, max 200).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Restrict the search to one session.
    #[serde(default)]
    pub session_id: Option<String>,
}

const LIST_DEFAULT: u32 = 50;
const LIST_MAX: u32 = 500;
const PAGE_DEFAULT: u32 = 500;
const PAGE_MAX: u32 = 2000;
const SEARCH_DEFAULT: u32 = 50;
const SEARCH_MAX: u32 = 200;

#[tool_router]
impl DarricService {
    pub fn new(db: Arc<DbConn>, live: Arc<dyn LiveStatus>) -> Self {
        Self {
            db,
            live,
            tool_router: Self::tool_router(),
        }
    }

    /// Run a query on the read-only connection off the async runtime.
    ///
    /// rusqlite blocks, and this runtime also drives audio capture, so every
    /// query goes through `spawn_blocking` — see
    /// `2026-05-19-decision-spawn-blocking-for-rusqlite-tools`.
    async fn query<T, F>(&self, f: F) -> Result<T, McpError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            let conn =
                db.0.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            f(&conn)
        })
        .await
        .map_err(internal)?
        .map_err(internal)
    }

    #[tool(
        description = "Live recorder state: whether a recording is running, which session it is, \
                       which devices are capturing, and how many audio segments have been dropped. \
                       Note: for the few seconds after Stop, `recording` is already false while \
                       the session still shows `in_progress` in `list_sessions`, because the \
                       final lines are still being transcribed."
    )]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        let snapshot = self.live.snapshot();
        let session = match snapshot.session_id.clone() {
            Some(id) => self
                .query(move |conn| sessions::get_session(conn, &id))
                .await?
                .map(|s| session_json(&s)),
            None => None,
        };
        json_result(&serde_json::json!({
            "recording": snapshot.session_id.is_some(),
            "session": session,
            "devices": snapshot.devices,
            "dropped_segments": snapshot.dropped_segments,
        }))
    }

    #[tool(
        description = "List recordings, newest first. Each has `id`, `topic`, `started_at`, \
                       `ended_at`, `recorded_minutes`, and `in_progress` (true while still \
                       recording). Use `get_transcript` with an `id` to read one."
    )]
    async fn list_sessions(
        &self,
        Parameters(args): Parameters<ListSessionsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(LIST_DEFAULT).clamp(1, LIST_MAX);
        let offset = args.offset.unwrap_or(0);
        let list = self
            .query(move |conn| sessions::list_sessions(conn, Some(limit), offset))
            .await?;
        json_result(&serde_json::json!({
            "sessions": list.iter().map(session_json).collect::<Vec<_>>(),
        }))
    }

    #[tool(
        description = "Read a recording's transcript as device-attributed lines, paged by a cursor. \
                       Each line has `seq`, `device_name`, `direction` (input = a microphone, output \
                       = what the machine played), `content`, and `recorded_at`. Lines come in \
                       transcription order, which can differ from speech order across devices — \
                       sort by `recorded_at` if that matters. Pass `next_cursor` back as `after` \
                       to fetch only what has landed since, including during a live recording. \
                       Cursors are valid only while darric stays running; do not persist them."
    )]
    async fn get_transcript(
        &self,
        Parameters(args): Parameters<GetTranscriptArgs>,
    ) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(PAGE_DEFAULT).clamp(1, PAGE_MAX);
        let id = args.session_id;
        let (session, page) = self
            .query(move |conn| {
                let session = sessions::get_session(conn, &id)?;
                let page = match session {
                    Some(_) => Some(sessions::transcript_page(conn, &id, args.after, limit)?),
                    None => None,
                };
                Ok((session, page))
            })
            .await?;
        let (Some(session), Some(page)) = (session, page) else {
            return Err(McpError::invalid_params("no such session", None));
        };
        json_result(&serde_json::json!({
            "session": session_json(&session),
            "lines": page.lines,
            "next_cursor": page.next_cursor,
            "has_more": page.has_more,
        }))
    }

    #[tool(
        description = "Case-insensitive substring search across every recording. Returns `lines` \
                       whose spoken content matched (newest first, each with its `session_id` and \
                       `seq`) and `sessions` whose topic matched. Device names are not searched. \
                       To read around a hit, call `get_transcript` with `after` set a few below \
                       the hit's `seq`."
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let query = args.query.trim().to_owned();
        if query.is_empty() {
            return Err(McpError::invalid_params("query must not be empty", None));
        }
        let limit = args.limit.unwrap_or(SEARCH_DEFAULT).clamp(1, SEARCH_MAX);
        let session_id = args.session_id;
        let results = self
            .query(move |conn| sessions::search(conn, &query, session_id.as_deref(), limit))
            .await?;
        json_result(&serde_json::json!({
            "sessions": results.sessions.iter().map(session_json).collect::<Vec<_>>(),
            "lines": results.lines,
        }))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DarricService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("darric", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Read-only access to darric's recordings: transcripts attributed to the device \
                 they came from, including a recording still in progress. Start with `status` \
                 to find the live session or `list_sessions` for past ones, read with \
                 `get_transcript`, and use `search` to find where something was said.",
            )
    }
}

fn session_json(session: &Session) -> serde_json::Value {
    serde_json::json!({
        "id": session.id,
        "topic": session.topic,
        "started_at": session.started_at,
        "ended_at": session.ended_at,
        "recorded_minutes": session.recorded_minutes,
        "in_progress": session.in_progress(),
    })
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

fn internal<E: std::fmt::Display>(err: E) -> McpError {
    McpError::internal_error(err.to_string(), None)
}

use crate::state::DbConn;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

mod queries;

/// Maximum number of transcript bytes returned by `get_meeting`. Beyond this,
/// the transcript is truncated and a `truncated_at_bytes` field is set so the
/// caller knows there is more content.
const MAX_TRANSCRIPT_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct DarricService {
    db: Arc<DbConn>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListNotesArgs {
    /// Maximum number of notes to return (default 50, max 500).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Number of notes to skip from the start of the result set (default 0).
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNoteArgs {
    /// Note id (UUID).
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Substring to match (case-insensitive). Required, non-empty.
    pub query: String,
    /// Maximum number of results per entity type (default 20, max 100).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListMeetingsArgs {
    /// Maximum number of meetings to return (default 50, max 500).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Inclusive lower bound on `started_at` as ISO-8601 / RFC-3339.
    #[serde(default)]
    pub since: Option<String>,
    /// Inclusive upper bound on `started_at` as ISO-8601 / RFC-3339.
    #[serde(default)]
    pub until: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMeetingArgs {
    /// Session/meeting id (UUID).
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTasksArgs {
    /// Filter by column: `todo`, `doing`, or `done`. Omit for all.
    #[serde(default)]
    pub status: Option<String>,
    /// Filter by tag name. Omit for all.
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ByTagArgs {
    /// Tag name to match (exact, case-sensitive).
    pub tag: String,
    /// Entity types to include: any of `notes`, `meetings`, `tasks`.
    /// Omit for all three.
    #[serde(default)]
    pub types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TimelineArgs {
    /// Inclusive lower bound (ISO-8601 / RFC-3339).
    #[serde(default)]
    pub from: Option<String>,
    /// Inclusive upper bound (ISO-8601 / RFC-3339).
    #[serde(default)]
    pub to: Option<String>,
    /// Entity types to include: any of `notes`, `meetings`, `tasks`, `chat`.
    /// Omit for all.
    #[serde(default)]
    pub types: Option<Vec<String>>,
    /// Maximum number of entries to return (default 100, max 1000).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[tool_router]
impl DarricService {
    pub fn new(db: Arc<DbConn>) -> Self {
        Self {
            db,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List notes ordered by most-recently-updated. Returns metadata and tags; \
                       use `get_note` to fetch the body."
    )]
    async fn list_notes(
        &self,
        Parameters(args): Parameters<ListNotesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::list_notes(&db, &args))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(description = "Fetch a single note by id, including full body and tags.")]
    async fn get_note(
        &self,
        Parameters(args): Parameters<GetNoteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::get_note(&db, &args.id))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "Substring search across notes (title + body), meeting topics + transcripts, \
                       and task titles. Case-insensitive."
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let query = args.query.trim().to_owned();
        if query.is_empty() {
            return Err(McpError::invalid_params("query must not be empty", None));
        }
        let db = self.db.clone();
        let limit = args.limit.unwrap_or(20).min(100);
        let value = tokio::task::spawn_blocking(move || queries::search(&db, &query, limit))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "List meetings (recording sessions) ordered by most-recent first. \
                       Returns topic, timing, recorded minutes, and tags."
    )]
    async fn list_meetings(
        &self,
        Parameters(args): Parameters<ListMeetingsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::list_meetings(&db, args))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "Fetch a single meeting by id, including the full transcript and any \
                       free-form notes. Transcripts beyond ~64KB are truncated; check the \
                       `truncated_at_bytes` field."
    )]
    async fn get_meeting(
        &self,
        Parameters(args): Parameters<GetMeetingArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || {
            queries::get_meeting(&db, &args.id, MAX_TRANSCRIPT_BYTES)
        })
        .await
        .map_err(internal)?
        .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "List kanban tasks ordered by (column, position). Optionally filter by \
                       column (`todo` / `doing` / `done`) and/or a single tag name."
    )]
    async fn list_tasks(
        &self,
        Parameters(args): Parameters<ListTasksArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::list_tasks(&db, args))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(description = "List all tags alphabetically.")]
    async fn list_tags(&self) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::list_tags(&db))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "Return notes, meetings, and tasks tagged with the given tag. Filter \
                       which entity types are included via the optional `types` array."
    )]
    async fn by_tag(
        &self,
        Parameters(args): Parameters<ByTagArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::by_tag(&db, &args))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }

    #[tool(
        description = "Chronological combined view across notes, meetings, tasks (and optionally \
                       chat messages). Newest first. Bounded by `from` / `to` and `limit`."
    )]
    async fn timeline(
        &self,
        Parameters(args): Parameters<TimelineArgs>,
    ) -> Result<CallToolResult, McpError> {
        let db = self.db.clone();
        let value = tokio::task::spawn_blocking(move || queries::timeline(&db, args))
            .await
            .map_err(internal)?
            .map_err(internal)?;
        json_result(&value)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DarricService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("darric", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Read-only access to darric's personal data: notes, meetings (with transcripts), \
                 tasks, tags, and a chronological timeline. All tools return JSON. \
                 Use `search` to find content across notes, meeting transcripts, and tasks.",
            )
    }
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value).map_err(internal)?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn internal<E: std::fmt::Display>(err: E) -> McpError {
    McpError::internal_error(err.to_string(), None)
}

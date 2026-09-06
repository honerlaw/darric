//! A read-only MCP server hosted by the running app.
//!
//! Streamable HTTP on a loopback port, so Claude Code and any other MCP client
//! on this machine can read recordings — including one still in progress —
//! without darric spending anything on AI itself. Loopback-only, and the tools
//! query through a read-only SQLite connection, so nothing an agent does can
//! write to the database or wait on the recorder.

pub mod service;

use crate::error::{AppError, Result};
use crate::state::DbConn;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use service::{DarricService, LiveStatus};
use std::future::Future;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The fixed loopback port. Chosen once in May 2026 and kept, so a client
/// configured against the old server still connects.
pub const DEFAULT_PORT: u16 = 27842;

/// The path the MCP endpoint is mounted at.
pub const ENDPOINT: &str = "/mcp";

/// A running server. Dropping it stops serving.
pub struct McpServerHandle {
    pub port: u16,
    cancel: CancellationToken,
}

impl McpServerHandle {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}{ENDPOINT}", self.port)
    }
}

impl Drop for McpServerHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// What the app knows about its server, for the status command.
pub enum McpServerState {
    NotStarted,
    Listening(McpServerHandle),
    /// The port is held by another process. The app keeps running without the
    /// endpoint; the fix is on the user's side, so the UI names it.
    PortBusy(String),
    /// Anything else went wrong bringing the server up — the read-only
    /// database connection, adopting the listener. Also non-fatal.
    Failed(String),
}

/// Bind the loopback listener.
///
/// Synchronous, so `setup` learns whether the port was free before it returns
/// and can record the outcome without a "starting" state the UI would have to
/// poll through. The listener is non-blocking because Tokio adopts it later.
pub fn bind(port: u16) -> Result<std::net::TcpListener> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| AppError::McpServer(format!("bind 127.0.0.1:{port}: {e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| AppError::McpServer(format!("set_nonblocking: {e}")))?;
    Ok(listener)
}

/// Build the server over an already-bound listener.
///
/// Returns the handle and the serve future separately: the caller spawns the
/// future on whichever runtime it owns (Tauri's in the app, Tokio's in a test),
/// and Tokio adopts the listener inside that future, where a runtime is
/// guaranteed to exist.
pub fn serve(
    listener: std::net::TcpListener,
    db: Arc<DbConn>,
    live: Arc<dyn LiveStatus>,
) -> Result<(McpServerHandle, impl Future<Output = ()> + Send)> {
    let port = listener
        .local_addr()
        .map_err(|e| AppError::McpServer(format!("local_addr: {e}")))?
        .port();

    let cancel = CancellationToken::new();
    let service_cancel = cancel.child_token();

    // `StreamableHttpServerConfig::default()` allows only localhost Host
    // headers. That is the DNS-rebinding floor the 2026-05-19 decision set;
    // it is never disabled here.
    let http_service = StreamableHttpService::new(
        move || Ok(DarricService::new(Arc::clone(&db), Arc::clone(&live))),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(service_cancel),
    );
    let router = axum::Router::new().nest_service(ENDPOINT, http_service);

    let serve_cancel = cancel.clone();
    let future = async move {
        let listener = match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => listener,
            Err(e) => {
                log::error!("[mcp_server] adopting the listener failed: {e}");
                return;
            }
        };
        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            serve_cancel.cancelled().await;
        });
        if let Err(e) = server.await {
            log::error!("[mcp_server] serve error: {e}");
        }
    };

    log::info!("[mcp_server] listening on http://127.0.0.1:{port}{ENDPOINT}");
    Ok((McpServerHandle { port, cancel }, future))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::service::{LiveDevice, LiveSnapshot};
    use super::*;
    use rmcp::model::CallToolRequestParams;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::ServiceExt;
    use rusqlite::Connection;
    use std::sync::Mutex;

    const TS: &str = "2024-01-01T09:00:00Z";

    struct StubLive(Option<String>);

    impl LiveStatus for StubLive {
        fn snapshot(&self) -> LiveSnapshot {
            LiveSnapshot {
                session_id: self.0.clone(),
                devices: vec![LiveDevice {
                    id: "dev".into(),
                    name: "Dev".into(),
                    direction: "input".into(),
                    state: "active".into(),
                }],
                dropped_segments: 0,
            }
        }
    }

    fn seed_line(conn: &Connection, id: &str, content: &str) {
        conn.execute(
            "INSERT INTO transcript_lines(
                 id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES(?1, 'live', 'dev', 'Dev', 'input', ?2, ?3)",
            rusqlite::params![id, content, TS],
        )
        .unwrap();
    }

    fn text_json(result: &rmcp::model::CallToolResult) -> serde_json::Value {
        let text = result
            .content
            .iter()
            .find_map(|c| c.as_text().map(|t| t.text.clone()))
            .expect("a text block");
        serde_json::from_str(&text).unwrap()
    }

    async fn call(
        client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
        name: &'static str,
        args: serde_json::Value,
    ) -> serde_json::Value {
        let mut params = CallToolRequestParams::new(name);
        params.arguments = args.as_object().cloned();
        let result = client.call_tool(params).await.unwrap();
        assert_ne!(
            result.is_error,
            Some(true),
            "{name} returned an error: {result:?}"
        );
        text_json(&result)
    }

    #[test]
    fn bind_refuses_a_port_another_listener_holds() {
        // The port-busy success criterion: a second darric must not take the
        // port from the first, and the error must say which port so the chip's
        // title is actionable.
        let held = bind(0).unwrap();
        let port = held.local_addr().unwrap().port();

        let err = bind(port).unwrap_err().to_string();
        assert!(err.contains(&format!("bind 127.0.0.1:{port}")), "{err}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn round_trip_over_streamable_http_sees_concurrent_writes() {
        // The arrangement the app runs under: one writer connection that owns
        // the file, and the server's own read-only connection observing it.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("darric.db");
        let writer = crate::db::open_at(&path).unwrap();
        writer
            .execute(
                "INSERT INTO sessions(id, topic, started_at, created_at) VALUES('live', 'Standup', ?1, ?1)",
                rusqlite::params![TS],
            )
            .unwrap();
        seed_line(&writer, "l1", "one");
        seed_line(&writer, "l2", "two");
        seed_line(&writer, "l3", "three");

        let reader = crate::db::open_read_only(&path).unwrap();
        let listener = bind(0).unwrap();
        let (handle, future) = serve(
            listener,
            Arc::new(DbConn(Mutex::new(reader))),
            Arc::new(StubLive(Some("live".into()))),
        )
        .unwrap();
        tokio::spawn(future);

        let transport = StreamableHttpClientTransport::with_client(
            reqwest13::Client::new(),
            StreamableHttpClientTransportConfig::with_uri(handle.url()),
        );
        let client = ().serve(transport).await.unwrap();

        let mut names: Vec<_> = client
            .list_all_tools()
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["get_transcript", "list_sessions", "search", "status"]
        );

        let status = call(&client, "status", serde_json::json!({})).await;
        assert_eq!(status["recording"], true);
        assert_eq!(status["session"]["id"], "live");
        assert_eq!(status["session"]["topic"], "Standup");
        assert_eq!(status["devices"][0]["name"], "Dev");

        let sessions = call(&client, "list_sessions", serde_json::json!({})).await;
        assert_eq!(sessions["sessions"][0]["id"], "live");
        assert_eq!(sessions["sessions"][0]["in_progress"], true);

        let page = call(
            &client,
            "get_transcript",
            serde_json::json!({ "session_id": "live", "limit": 10 }),
        )
        .await;
        assert_eq!(page["lines"].as_array().unwrap().len(), 3);
        assert_eq!(page["has_more"], false);
        let cursor = page["next_cursor"].as_i64().unwrap();

        // A line lands mid-meeting through the writer while the server is up.
        // The read-only handle must see it, and the cursor must return exactly
        // it — this is the live-recording criterion, exercised.
        seed_line(&writer, "l4", "four");
        let page = call(
            &client,
            "get_transcript",
            serde_json::json!({ "session_id": "live", "after": cursor }),
        )
        .await;
        let contents: Vec<_> = page["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["content"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(contents, ["four"]);

        let hits = call(&client, "search", serde_json::json!({ "query": "thr" })).await;
        assert_eq!(hits["lines"][0]["content"], "three");
        assert_eq!(hits["lines"][0]["session_id"], "live");

        let unknown = client
            .call_tool({
                let mut p = CallToolRequestParams::new("get_transcript");
                p.arguments = serde_json::json!({ "session_id": "nope" })
                    .as_object()
                    .cloned();
                p
            })
            .await;
        assert!(
            unknown.is_err(),
            "an unknown session is a protocol error, not an empty page"
        );

        client.cancel().await.unwrap();
        drop(handle);
    }
}

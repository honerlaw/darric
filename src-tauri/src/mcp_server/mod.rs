use crate::error::{AppError, Result};
use crate::state::DbConn;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub mod service;
pub use service::DarricService;

pub struct McpServerHandle {
    pub bound_port: u16,
    cancel: CancellationToken,
}

impl Drop for McpServerHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Bind a TCP listener on `127.0.0.1:port` (use `0` for OS-assigned), wire up
/// the MCP service over Streamable HTTP at `/mcp`, and spawn a tokio task that
/// serves until the returned handle is dropped or `shutdown()` is called.
pub async fn spawn(db: Arc<DbConn>, port: u16) -> Result<McpServerHandle> {
    let addr = format!("127.0.0.1:{port}");
    let tcp_listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| AppError::McpServer(format!("bind {addr}: {e}")))?;
    let bound_port = tcp_listener
        .local_addr()
        .map_err(|e| AppError::McpServer(format!("local_addr: {e}")))?
        .port();

    let cancel = CancellationToken::new();
    let service_cancel = cancel.child_token();

    let db_for_factory = db.clone();
    let http_service = StreamableHttpService::new(
        move || Ok(DarricService::new(db_for_factory.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(service_cancel),
    );

    let router = axum::Router::new().nest_service("/mcp", http_service);

    let serve_cancel = cancel.clone();
    tokio::spawn(async move {
        let server = axum::serve(tcp_listener, router).with_graceful_shutdown(async move {
            serve_cancel.cancelled().await;
        });
        if let Err(e) = server.await {
            log::error!("[mcp_server] serve error: {e}");
        }
    });

    log::info!("[mcp_server] listening on http://127.0.0.1:{bound_port}/mcp");
    Ok(McpServerHandle { bound_port, cancel })
}

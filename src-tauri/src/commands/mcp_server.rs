use crate::{
    audio::CaptureEngine,
    error::Result,
    mcp_server::{
        service::{LiveDevice, LiveSnapshot, LiveStatus},
        McpServerState,
    },
    state::AppState,
};
use tauri::{AppHandle, Manager};

#[derive(Debug, serde::Serialize)]
pub struct McpServerStatus {
    pub listening: bool,
    /// The bound port, or the port that could not be bound.
    pub port: u16,
    pub url: Option<String>,
    /// Another process holds the port — the one failure the user can fix.
    pub port_busy: bool,
    pub error: Option<String>,
}

/// What the header chip shows. Polled once on mount rather than pushed: the
/// outcome is decided in `setup`, and an event emitted there reaches no webview.
#[tauri::command]
pub async fn mcp_server_status(state: tauri::State<'_, AppState>) -> Result<McpServerStatus> {
    let server = state
        .mcp_server
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(match &*server {
        McpServerState::Listening(handle) => McpServerStatus {
            listening: true,
            port: handle.port,
            url: Some(handle.url()),
            port_busy: false,
            error: None,
        },
        McpServerState::PortBusy(reason) => McpServerStatus {
            listening: false,
            port: crate::mcp_server::DEFAULT_PORT,
            url: None,
            port_busy: true,
            error: Some(reason.clone()),
        },
        McpServerState::Failed(reason) => McpServerStatus {
            listening: false,
            port: crate::mcp_server::DEFAULT_PORT,
            url: None,
            port_busy: false,
            error: Some(reason.clone()),
        },
        McpServerState::NotStarted => McpServerStatus {
            listening: false,
            port: crate::mcp_server::DEFAULT_PORT,
            url: None,
            port_busy: false,
            error: None,
        },
    })
}

/// The recorder as the MCP `status` tool sees it: a short read of the engine
/// under `AppState`, taken through the app handle so the service itself never
/// holds a reference into Tauri state.
pub struct AppLiveStatus(pub AppHandle);

impl LiveStatus for AppLiveStatus {
    fn snapshot(&self) -> LiveSnapshot {
        let state = self.0.state::<AppState>();
        let engine = state
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        engine.as_ref().map_or(
            LiveSnapshot {
                session_id: None,
                devices: Vec::new(),
                dropped_segments: 0,
            },
            |engine: &CaptureEngine| LiveSnapshot {
                session_id: Some(engine.session_id().to_string()),
                devices: engine
                    .statuses()
                    .into_iter()
                    .map(|s| LiveDevice {
                        id: s.device.id,
                        name: s.device.name,
                        direction: s.device.direction.as_str().to_string(),
                        state: s.state.as_str().to_string(),
                    })
                    .collect(),
                dropped_segments: engine.dropped_segments(),
            },
        )
    }
}

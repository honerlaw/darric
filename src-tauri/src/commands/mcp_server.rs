use crate::{error::Result, state::AppState};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct McpServerStatus {
    pub enabled: bool,
    pub configured_port: u16,
    pub listening: bool,
    pub bound_port: Option<u16>,
    pub url: Option<String>,
}

#[tauri::command]
pub async fn mcp_server_status(state: tauri::State<'_, AppState>) -> Result<McpServerStatus> {
    let (enabled, configured_port) = {
        let conn = state.db.0.lock().expect("db mutex poisoned");
        let enabled =
            crate::db::get_setting(&conn, "mcp_server.enabled").as_deref() != Some("false");
        let configured_port = crate::db::get_setting(&conn, "mcp_server.port")
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(27842);
        (enabled, configured_port)
    };

    let bound_port = state
        .mcp_server
        .lock()
        .expect("mcp_server mutex poisoned")
        .as_ref()
        .map(|h| h.bound_port);

    let url = bound_port.map(|p| format!("http://127.0.0.1:{p}/mcp"));

    Ok(McpServerStatus {
        enabled,
        configured_port,
        listening: bound_port.is_some(),
        bound_port,
        url,
    })
}

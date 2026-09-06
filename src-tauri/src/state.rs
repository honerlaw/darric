use crate::audio::device::ExclusionRegistry;
use crate::audio::CaptureEngine;
use crate::mcp_server::McpServerState;
use crate::transcription::loader::TranscriberSlot;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct DbConn(pub Mutex<rusqlite::Connection>);

pub struct AppState {
    pub db: Arc<DbConn>,
    /// The running capture session, if any.
    pub engine: Mutex<Option<CaptureEngine>>,
    pub transcriber: TranscriberSlot,
    /// Serialises `start_session` / `resume_session`.
    ///
    /// The `engine.is_some()` guard alone is a check-then-act: the engine is not
    /// installed until capture has actually started, and starting now awaits the
    /// model load, so two commands could both pass the guard, both build an
    /// engine, and have the second overwrite the first. `CaptureEngine` has no
    /// `Drop`, so the overwritten one's capture threads and whisper workers
    /// would run — and keep writing transcript lines — until the app quit.
    pub session_transition: tokio::sync::Mutex<()>,
    /// Aggregate devices this process created for output taps, which must never
    /// be enumerated back as inputs.
    pub exclusions: ExclusionRegistry,
    /// Device ids the user has switched OFF. Everything discovered is captured
    /// by default — "record everything" is the point — so this stores the
    /// exceptions rather than the selections, and a newly plugged-in device is
    /// therefore live without needing to be enabled by hand.
    pub disabled_devices: Mutex<HashSet<String>>,
    /// The loopback MCP server, or why it is not listening. Set once in
    /// `setup`; the handle stops the server when the state is dropped.
    pub mcp_server: Mutex<McpServerState>,
}

impl AppState {
    pub fn new(conn: rusqlite::Connection, disabled_devices: HashSet<String>) -> Self {
        Self {
            db: Arc::new(DbConn(Mutex::new(conn))),
            engine: Mutex::new(None),
            transcriber: Arc::new(tokio::sync::Mutex::new(None)),
            session_transition: tokio::sync::Mutex::new(()),
            exclusions: ExclusionRegistry::new(),
            disabled_devices: Mutex::new(disabled_devices),
            mcp_server: Mutex::new(McpServerState::NotStarted),
        }
    }
}

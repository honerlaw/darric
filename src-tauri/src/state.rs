use crate::ai::{harness::AgentHarness, mcp::McpManager, ChatMessage};
use crate::mcp_server::McpServerHandle;
use crate::transcription::{speaker_tracker::SpeakerTracker, Transcriber};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

pub struct DbConn(pub Mutex<rusqlite::Connection>);

pub struct AudioHandle {
    pub session_id: String,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct AppState {
    pub db: Arc<DbConn>,
    pub audio: Mutex<Option<AudioHandle>>,
    pub transcriber: Arc<Mutex<Option<Arc<Transcriber>>>>,
    pub speaker_tracker: Arc<Mutex<SpeakerTracker>>,
    pub ai_harness: Arc<AsyncMutex<Option<AgentHarness>>>,
    pub chat_history: Arc<AsyncMutex<Vec<ChatMessage>>>,
    pub mcp: Arc<McpManager>,
    pub mcp_server: Arc<Mutex<Option<McpServerHandle>>>,
}

impl AppState {
    pub fn new(
        conn: rusqlite::Connection,
        harness: Option<AgentHarness>,
        history: Vec<ChatMessage>,
        mcp: Arc<McpManager>,
    ) -> Self {
        Self {
            db: Arc::new(DbConn(Mutex::new(conn))),
            audio: Mutex::new(None),
            transcriber: Arc::new(Mutex::new(None)),
            speaker_tracker: Arc::new(Mutex::new(SpeakerTracker::new())),
            ai_harness: Arc::new(AsyncMutex::new(harness)),
            chat_history: Arc::new(AsyncMutex::new(history)),
            mcp,
            mcp_server: Arc::new(Mutex::new(None)),
        }
    }
}

use crate::transcription::{speaker_tracker::SpeakerTracker, Transcriber};
use std::sync::{Arc, Mutex};

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
}

impl AppState {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self {
            db: Arc::new(DbConn(Mutex::new(conn))),
            audio: Mutex::new(None),
            transcriber: Arc::new(Mutex::new(None)),
            speaker_tracker: Arc::new(Mutex::new(SpeakerTracker::new())),
        }
    }
}

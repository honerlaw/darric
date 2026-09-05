use crate::audio::device::ExclusionRegistry;
use crate::audio::CaptureEngine;
use crate::transcription::loader::TranscriberSlot;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct DbConn(pub Mutex<rusqlite::Connection>);

pub struct AppState {
    pub db: Arc<DbConn>,
    /// The running capture session, if any.
    pub engine: Mutex<Option<CaptureEngine>>,
    pub transcriber: TranscriberSlot,
    /// Aggregate devices this process created for output taps, which must never
    /// be enumerated back as inputs.
    pub exclusions: ExclusionRegistry,
    /// Device ids the user has switched OFF. Everything discovered is captured
    /// by default — "record everything" is the point — so this stores the
    /// exceptions rather than the selections, and a newly plugged-in device is
    /// therefore live without needing to be enabled by hand.
    pub disabled_devices: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new(conn: rusqlite::Connection, disabled_devices: HashSet<String>) -> Self {
        Self {
            db: Arc::new(DbConn(Mutex::new(conn))),
            engine: Mutex::new(None),
            transcriber: Arc::new(tokio::sync::Mutex::new(None)),
            exclusions: ExclusionRegistry::new(),
            disabled_devices: Mutex::new(disabled_devices),
        }
    }
}

//! The capture engine: N devices in, one transcript out.
//!
//! Every enabled device gets its own supervisor thread ([`source`]) and its own
//! [`segmenter::Segmenter`], so the devices are independent — one failing or
//! being unplugged does not disturb the others. Completed segments go to a
//! shared [`crate::transcription::pool::TranscriptionPool`], which is where
//! backpressure is absorbed.

pub mod device;
pub mod resample;
pub mod segmenter;
pub mod source;

use crate::error::Result;
use crate::state::DbConn;
use crate::transcription::pool::{SegmentJob, TranscribedLine, TranscriptionPool};
use crate::transcription::Transcriber;
use device::CaptureDevice;
use segmenter::Segmenter;
use source::{SharedStatus, SourceStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter};

/// Segments held per source before the oldest is dropped.
///
/// Four segments is ~32 s of audio. Beyond that the transcript is so far behind
/// real time that catching up matters more than completeness — and the drop is
/// counted and surfaced either way.
const QUEUE_CAPACITY_PER_SOURCE: usize = 4;

/// A running capture session across every enabled device.
pub struct CaptureEngine {
    session_id: String,
    shutdown: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    statuses: Vec<SharedStatus>,
    segmenters: Vec<(CaptureDevice, Arc<Mutex<Segmenter>>)>,
    pool: Option<Arc<TranscriptionPool>>,
}

impl CaptureEngine {
    /// Start capturing every device in `devices`.
    ///
    /// With no transcriber the audio is still captured and metered — the UI
    /// stays honest about which devices are live while the model loads — but
    /// nothing is transcribed.
    pub fn start(
        session_id: String,
        devices: Vec<CaptureDevice>,
        transcriber: Option<Arc<Transcriber>>,
        worker_count: usize,
        app: &AppHandle,
        db: &Arc<DbConn>,
    ) -> Result<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));

        let pool = transcriber.map(|t| {
            let sink_db = Arc::clone(db);
            let sink_app = app.clone();
            let sink_session = session_id.clone();
            let sink: crate::transcription::pool::LineSink =
                Arc::new(move |line: TranscribedLine| {
                    persist_and_emit(&sink_db, &sink_app, &sink_session, &line);
                });
            Arc::new(TranscriptionPool::new(
                &t,
                worker_count,
                QUEUE_CAPACITY_PER_SOURCE * devices.len().max(1),
                &sink,
            ))
        });
        if pool.is_none() {
            log::warn!("[audio] no transcriber — capturing without transcription");
        }
        let pool: Option<Arc<TranscriptionPool>> = pool;

        let mut threads = Vec::new();
        let mut statuses = Vec::new();
        let mut segmenters = Vec::new();

        for dev in devices {
            let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(dev.clone())));
            let seg = Arc::new(Mutex::new(Segmenter::new()));
            statuses.push(Arc::clone(&status));
            segmenters.push((dev.clone(), Arc::clone(&seg)));

            let pool_for_source = pool.clone();
            let dev_for_cb = dev.clone();
            let shutdown_for_source = Arc::clone(&shutdown);

            let handle = std::thread::Builder::new()
                .name(format!("capture-{}", dev.name))
                .spawn(move || {
                    source::run_source(&dev_for_cb.clone(), &status, &shutdown_for_source, {
                        let seg = Arc::clone(&seg);
                        let pool = pool_for_source.clone();
                        move |samples| {
                            // Short, effectively uncontended critical section: the
                            // only other holder is the flush at stop.
                            let ready = {
                                let mut s = seg
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                s.push(samples)
                            };
                            for segment in ready {
                                if let Some(p) = pool.as_ref() {
                                    p.submit(job(&dev_for_cb, segment));
                                }
                            }
                        }
                    });
                })
                .map_err(|e| crate::error::AppError::Audio(e.to_string()));

            match handle {
                Ok(h) => threads.push(h),
                Err(e) => {
                    // Stop and join whatever already started, or those threads
                    // run forever with no handle able to reach them.
                    log::error!("[audio] failed to spawn a capture thread: {e}");
                    shutdown.store(true, Ordering::SeqCst);
                    for t in threads {
                        t.join().ok();
                    }
                    if let Some(p) = pool.as_ref() {
                        p.shutdown();
                    }
                    return Err(e);
                }
            }
        }

        Ok(Self {
            session_id,
            shutdown,
            threads,
            statuses,
            segmenters,
            pool,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Snapshot of every source, for the UI's device rows.
    pub fn statuses(&self) -> Vec<SourceStatus> {
        self.statuses
            .iter()
            .map(|s| {
                s.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone()
            })
            .collect()
    }

    /// Segments discarded because transcription fell behind.
    pub fn dropped_segments(&self) -> u64 {
        self.pool.as_ref().map_or(0, |p| p.dropped())
    }

    /// Stop every source, flush trailing partial segments, and drain the pool.
    pub fn stop(self) {
        self.shutdown.store(true, Ordering::SeqCst);
        for t in self.threads {
            t.join().ok();
        }

        // Flush after the sources are stopped so nothing races the final push.
        // Every capture thread has been joined by now, so their pool clones are
        // dropped and this is the last reference outside the workers themselves.
        if let Some(pool) = self.pool.as_ref() {
            for (dev, seg) in &self.segmenters {
                let tail = seg
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .flush();
                if let Some(samples) = tail {
                    log::info!(
                        "[audio] flushing {} trailing samples for {}",
                        samples.len(),
                        dev.name
                    );
                    pool.submit(job(dev, samples));
                }
            }
        }

        if let Some(pool) = self.pool.as_ref() {
            pool.shutdown();
            let dropped = pool.dropped();
            if dropped > 0 {
                log::warn!("[audio] {dropped} segment(s) were dropped this session");
            }
        }
    }
}

fn job(device: &CaptureDevice, samples: Vec<f32>) -> SegmentJob {
    SegmentJob {
        device_id: device.id.clone(),
        device_name: device.name.clone(),
        direction: device.direction,
        samples,
    }
}

fn persist_and_emit(db: &Arc<DbConn>, app: &AppHandle, session_id: &str, line: &TranscribedLine) {
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    {
        let conn =
            db.0.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(e) = conn.execute(
            "INSERT INTO transcript_lines(
                 id, session_id, device_id, device_name, direction, content, recorded_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                session_id,
                line.device_id,
                line.device_name,
                line.direction.as_str(),
                line.text,
                now
            ],
        ) {
            log::error!("[audio] failed to persist transcript line: {e}");
        }
    }
    app.emit(
        "transcript_chunk",
        serde_json::json!({
            "device_id": line.device_id,
            "device_name": line.device_name,
            "direction": line.direction.as_str(),
            "content": line.text,
            "recorded_at": now,
        }),
    )
    .ok();
}

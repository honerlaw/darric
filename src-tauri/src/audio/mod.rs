//! The capture engine: N devices in, one transcript out.
//!
//! Every enabled device gets its own supervisor thread ([`source`]) and its own
//! [`segmenter::Segmenter`], so the devices are independent — one failing or
//! being unplugged does not disturb the others. Completed segments go to a
//! shared [`crate::transcription::pool::TranscriptionPool`], which is where
//! backpressure is absorbed.

pub mod coreaudio;
pub mod device;
pub mod resample;
pub mod segmenter;
pub mod source;
pub mod tap;

use crate::error::Result;
use crate::state::DbConn;
use crate::transcription::pool::{SegmentJob, TranscribedLine, TranscriptionPool};
use crate::transcription::Transcriber;
use device::{CaptureDevice, ExclusionRegistry};
use segmenter::Segmenter;
use source::{SharedStatus, SourceState, SourceStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tap::OutputTap;
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
    /// Live output taps. Dropping these stops and destroys the underlying Core
    /// Audio objects, so they are held for the session's lifetime.
    taps: Vec<OutputTap>,
    exclusions: ExclusionRegistry,
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
        exclusions: &ExclusionRegistry,
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
        let mut taps = Vec::new();

        let (outputs, inputs): (Vec<_>, Vec<_>) = devices
            .into_iter()
            .partition(|d| d.direction == crate::transcription::pool::Direction::Output);

        // Output devices are captured through a Core Audio process tap rather
        // than a cpal stream. Core Audio drives the callback itself, so these
        // need no supervisor thread — only somewhere to live until stop.
        for dev in outputs {
            let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(dev.clone())));
            let seg = Arc::new(Mutex::new(Segmenter::new()));
            statuses.push(Arc::clone(&status));
            segmenters.push((dev.clone(), Arc::clone(&seg)));

            let pool_for_tap = pool.clone();
            let dev_for_cb = dev.clone();
            let status_for_cb = Arc::clone(&status);
            let sink = move |samples: &[f32]| {
                if let Ok(mut st) = status_for_cb.try_lock() {
                    st.level = resample::rms(samples);
                }
                let ready = {
                    let mut sg = seg
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    sg.push(samples)
                };
                for segment in ready {
                    if let Some(p) = pool_for_tap.as_ref() {
                        p.submit(job(&dev_for_cb, segment));
                    }
                }
            };

            match OutputTap::start(&dev.id, &dev.name, sink) {
                Ok(t) => {
                    // Register before anything can enumerate again, or the app
                    // lists its own tap as a microphone and records itself.
                    exclusions.register(t.aggregate_uid());
                    let mut st = status
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    st.state = SourceState::Active;
                    drop(st);
                    log::info!("[audio] tapping output device {}", dev.name);
                    taps.push(t);
                }
                Err(e) => {
                    // One device failing must not sink the recording.
                    log::error!("[audio] could not tap {}: {e}", dev.name);
                    let mut st = status
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    st.state = SourceState::Failed;
                }
            }
        }

        for dev in inputs {
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
            taps,
            exclusions: exclusions.clone(),
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
    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);

        // Drop the taps first: this stops their IOProcs, so no further samples
        // arrive while the segmenters are being flushed below.
        for t in self.taps.drain(..) {
            self.exclusions.unregister(t.aggregate_uid());
            drop(t);
        }

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

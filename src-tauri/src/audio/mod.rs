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
use segmenter::{Segment, Segmenter};
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
    pool: Arc<TranscriptionPool>,
    /// Live output taps. Dropping these stops and destroys the underlying Core
    /// Audio objects, so they are held for the session's lifetime.
    taps: Vec<OutputTap>,
    exclusions: ExclusionRegistry,
}

impl CaptureEngine {
    /// Start capturing every device in `devices`.
    ///
    /// A transcriber is required, not optional. It used to be an `Option`, and a
    /// caller that silently passed `None` produced a session that captured and
    /// metered audio while transcribing none of it — the bug this signature now
    /// makes unexpressible. Callers that cannot obtain a transcriber must fail
    /// rather than start a recording that cannot produce anything.
    pub fn start(
        session_id: String,
        devices: Vec<CaptureDevice>,
        transcriber: &Arc<Transcriber>,
        worker_count: usize,
        app: &AppHandle,
        db: &Arc<DbConn>,
        exclusions: &ExclusionRegistry,
    ) -> Result<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));

        let pool = {
            let sink_db = Arc::clone(db);
            let sink_app = app.clone();
            let sink_session = session_id.clone();
            let sink: crate::transcription::pool::LineSink =
                Arc::new(move |line: TranscribedLine| {
                    persist_and_emit(&sink_db, &sink_app, &sink_session, &line);
                });
            Arc::new(TranscriptionPool::new(
                transcriber,
                worker_count,
                QUEUE_CAPACITY_PER_SOURCE * devices.len().max(1),
                &sink,
            ))
        };

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

            let pool_for_tap = Arc::clone(&pool);
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
                    pool_for_tap.submit(job(&dev_for_cb, segment));
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

            let pool_for_source = Arc::clone(&pool);
            let dev_for_cb = dev.clone();
            let shutdown_for_source = Arc::clone(&shutdown);

            let handle = std::thread::Builder::new()
                .name(format!("capture-{}", dev.name))
                .spawn(move || {
                    source::run_source(&dev_for_cb.clone(), &status, &shutdown_for_source, {
                        let seg = Arc::clone(&seg);
                        let pool = Arc::clone(&pool_for_source);
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
                                pool.submit(job(&dev_for_cb, segment));
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
                    pool.shutdown();
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
        self.pool.dropped()
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
        for (dev, seg) in &self.segmenters {
            let tail = seg
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .flush();
            if let Some(segment) = tail {
                log::info!(
                    "[audio] flushing {} trailing samples for {}",
                    segment.samples.len(),
                    dev.name
                );
                self.pool.submit(job(dev, segment));
            }
        }

        self.pool.shutdown();
        let dropped = self.pool.dropped();
        if dropped > 0 {
            log::warn!("[audio] {dropped} segment(s) were dropped this session");
        }
    }
}

fn job(device: &CaptureDevice, segment: Segment) -> SegmentJob {
    SegmentJob {
        device_id: device.id.clone(),
        device_name: device.name.clone(),
        direction: device.direction,
        samples: segment.samples,
        captured_at: segment.captured_at,
    }
}

/// The `recorded_at` a line is stored and emitted with: when its audio was
/// captured, not when whisper finished with it. Transcription runs seconds
/// behind capture and finishes in a different order across devices, so the
/// capture time is what puts two devices' lines back into speech order.
fn recorded_at(line: &TranscribedLine) -> String {
    line.captured_at.to_rfc3339()
}

fn persist_and_emit(db: &Arc<DbConn>, app: &AppHandle, session_id: &str, line: &TranscribedLine) {
    let now = recorded_at(line);
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
    app.emit("transcript_chunk", chunk_payload(session_id, line))
        .ok();
}

/// The `transcript_chunk` event payload.
///
/// Split out from the emit so the wire contract is reachable from a test —
/// `AppHandle` is not constructible in one. `session_id` is the load-bearing
/// field: the frontend keeps this listener alive past the end of a recording to
/// catch whisper's asynchronous flush, and it filters on this id to tell a late
/// chunk for the session that just stopped from one for the session now on
/// screen. Dropping the field does not fail loudly — the frontend's filter would
/// match nothing and silently stop appending every live line.
fn chunk_payload(session_id: &str, line: &TranscribedLine) -> serde_json::Value {
    serde_json::json!({
        "session_id": session_id,
        "device_id": line.device_id,
        "device_name": line.device_name,
        "direction": line.direction.as_str(),
        "content": line.text,
        "recorded_at": recorded_at(line),
    })
}

/// The reported bug, against real hardware: every output device is tapped at
/// once, the spoken fixture is played through the default output, and only
/// that device's tap may produce a line. The taps that carried nothing used to
/// say "Thank you." every eight seconds.
///
/// Ignored by default: it needs the whisper model, macOS speech synthesis, the
/// audio-capture permission on the process running it, and a machine with at
/// least one output device. Run with:
///   cargo test --manifest-path src-tauri/Cargo.toml --lib taps_transcribe -- --ignored --nocapture
#[cfg(test)]
mod hardware {
    use super::*;
    use crate::transcription::fixture::{model_path, spoken_wav, EXPECTED_PHRASE};

    #[test]
    #[ignore = "taps real output devices and plays audio through the default one"]
    fn taps_transcribe_only_the_device_that_played() {
        let Some(path) = model_path() else {
            println!("no model present — skipping");
            return;
        };
        let transcriber = Transcriber::new(path.to_str().expect("utf-8 path")).expect("models");
        let (_dir, wav) = spoken_wav();

        let outputs = device::list_output_devices(&ExclusionRegistry::new());
        assert!(!outputs.is_empty(), "no output devices to tap");

        let mut taps = Vec::new();
        for dev in &outputs {
            let captured: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let sink_buf = Arc::clone(&captured);
            let tap = OutputTap::start(&dev.id, &dev.name, move |samples| {
                sink_buf
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend_from_slice(samples);
            })
            .unwrap_or_else(|e| panic!("tapping {}: {e}", dev.name));
            taps.push((dev.name.clone(), tap, captured));
        }

        // `afplay` goes to the default output and blocks until the file ends.
        let ok = std::process::Command::new("afplay")
            .arg(&wav)
            .stdin(std::process::Stdio::null())
            .status()
            .expect("run `afplay`")
            .success();
        assert!(ok, "`afplay` failed");
        std::thread::sleep(std::time::Duration::from_secs(1));

        let mut heard: Vec<(String, String)> = Vec::new();
        for (name, tap, captured) in taps {
            drop(tap);
            let samples = captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let line = transcriber.transcribe(&samples).expect("transcribe");
            println!("{name}: {} samples -> {line:?}", samples.len());
            if let Some(text) = line {
                heard.push((name, text));
            }
        }

        assert_eq!(
            heard.len(),
            1,
            "exactly one output device carried the speech: {heard:?}"
        );
        assert!(
            heard[0].1.to_lowercase().contains(EXPECTED_PHRASE),
            "the tap that heard it transcribed it: {heard:?}"
        );
    }
    /// The microphone half of the same check, through the production cpal
    /// supervisor and resampler: every input device is captured while the
    /// fixture plays through the default output, and at least one microphone
    /// must hear it and transcribe it. Needs a default output that a microphone
    /// can hear — the built-in speakers, not headphones.
    #[test]
    #[ignore = "captures real microphones while playing audio through the default output"]
    fn a_microphone_hears_what_the_speakers_play() {
        let Some(path) = model_path() else {
            println!("no model present — skipping");
            return;
        };
        let transcriber = Transcriber::new(path.to_str().expect("utf-8 path")).expect("models");
        let (_dir, wav) = spoken_wav();

        let inputs = device::list_input_devices(&ExclusionRegistry::new());
        assert!(!inputs.is_empty(), "no input devices to capture");

        let shutdown = Arc::new(AtomicBool::new(false));
        let mut sources = Vec::new();
        for dev in &inputs {
            let captured: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(dev.clone())));
            let sink_buf = Arc::clone(&captured);
            let dev_for_thread = dev.clone();
            let status_for_thread = Arc::clone(&status);
            let shutdown_for_thread = Arc::clone(&shutdown);
            let handle = std::thread::spawn(move || {
                source::run_source(
                    &dev_for_thread,
                    &status_for_thread,
                    &shutdown_for_thread,
                    move |samples| {
                        sink_buf
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .extend_from_slice(samples);
                    },
                );
            });
            sources.push((dev.name.clone(), handle, captured));
        }
        // Let the streams come up before the audio starts.
        std::thread::sleep(std::time::Duration::from_secs(2));

        let ok = std::process::Command::new("afplay")
            .arg(&wav)
            .stdin(std::process::Stdio::null())
            .status()
            .expect("run `afplay`")
            .success();
        assert!(ok, "`afplay` failed");
        std::thread::sleep(std::time::Duration::from_secs(1));
        shutdown.store(true, Ordering::SeqCst);

        let mut heard: Vec<(String, String)> = Vec::new();
        for (name, handle, captured) in sources {
            handle.join().expect("capture thread");
            let samples = captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let line = transcriber.transcribe(&samples).expect("transcribe");
            println!("{name}: {} samples -> {line:?}", samples.len());
            if let Some(text) = line {
                heard.push((name, text));
            }
        }

        assert!(
            heard
                .iter()
                .any(|(_, text)| text.to_lowercase().contains(EXPECTED_PHRASE)),
            "some microphone must have heard and transcribed the speech: {heard:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::pool::Direction;

    fn line() -> TranscribedLine {
        TranscribedLine {
            device_id: "dev-1".to_string(),
            device_name: "MacBook Microphone".to_string(),
            direction: Direction::Input,
            text: "hello there".to_string(),
            captured_at: chrono::DateTime::parse_from_rfc3339("2024-01-01T09:00:05+00:00")
                .expect("fixed timestamp")
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn recorded_at_is_the_capture_time_not_now() {
        // A line reaches the sink seconds after its audio was heard, and two
        // devices' lines arrive in whichever order whisper finished them.
        // Ordering by recorded_at only restores speech order if it is the
        // capture time.
        let stamp = recorded_at(&line());
        assert_eq!(stamp, "2024-01-01T09:00:05+00:00");
        assert_eq!(chunk_payload("s", &line())["recorded_at"], stamp);
    }

    #[test]
    fn chunk_payload_names_its_session() {
        // The frontend drops any chunk whose session_id does not match the
        // session on screen. Without this field every live line is filtered out
        // and the transcript silently stops updating - no error anywhere.
        let payload = chunk_payload("session-42", &line());

        assert_eq!(payload["session_id"], "session-42");
    }

    #[test]
    fn chunk_payload_carries_the_fields_the_frontend_reads() {
        // TranscriptChunk in src/types/index.ts. A renamed or dropped key here
        // reaches the UI as undefined rather than as a build failure.
        let payload = chunk_payload("session-42", &line());

        assert_eq!(payload["device_id"], "dev-1");
        assert_eq!(payload["device_name"], "MacBook Microphone");
        assert_eq!(payload["direction"], "input");
        assert_eq!(payload["content"], "hello there");
        assert_eq!(payload["recorded_at"], "2024-01-01T09:00:05+00:00");
    }
}
